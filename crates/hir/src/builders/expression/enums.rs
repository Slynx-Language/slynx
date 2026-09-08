use common::{Span, Spanned, pool::DedupPoolId};
use slynx_parser::{ASTExpression, Type, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, HirType, Result, SymbolPointer,
    builders::HirQueueBuilder, generics::GenericTypeArguments,
};

use super::{ExpressionBuilder, ExpressionDescriptor};

pub struct EnumVariantDescriptor<'a> {
    pub variant_id: usize,
    pub arguments: &'a [Spanned<DedupPoolId<ASTExpression>>],
}

pub struct EnumExpressionDescriptor<'a> {
    ///A reference to the enum's HIR type id.
    pub enum_type: DedupPoolId<HirType>,
    pub variant: EnumVariantDescriptor<'a>,
    ///The generic type parameters passed when creating this enum expression. Such as 'Option.Some<int>(65);' explicitly provides [int]
    pub generics: &'a [Spanned<DedupPoolId<Type>>],
    pub span: Span,
    pub context: &'a TypeContext<'a>,
}

impl ExpressionBuilder {
    ///Resolves an enum variant by `name` within the current file (and the
    ///modules it imports). Returns the enum's HIR type id and the index of the
    ///variant within that enum, if a variant with the given name is declared
    ///somewhere reachable.
    pub(super) fn resolve_enum_variant(
        &self,
        queue: &HirQueueBuilder,
        name: SymbolPointer,
    ) -> Option<(DedupPoolId<HirType>, usize)> {
        let (owner, enum_id, variant_index) = queue.modules.find_enum_variant(name, self.file())?;
        let node = queue.get_node(owner);
        let enum_decl = queue.modules.get_entry(owner).enums().get(enum_id);
        let context = TypeContext::new(&enum_decl.type_params);
        let (_, enum_ty) = node
            .find_type_named_as(enum_decl.span.make_spanned(enum_decl.name), &context)
            .ok()?;
        Some((enum_ty, variant_index))
    }

    pub(super) fn build_enum_expression(
        &mut self,
        queue: &HirQueueBuilder,
        descriptor: EnumExpressionDescriptor,
    ) -> Result<HirExpression> {
        let enum_type_view = queue.hir.view(descriptor.enum_type);
        let enum_type = enum_type_view.dereference();
        let enum_view = enum_type
            .is_enum()
            .expect("Expected type given on building enum expression to be an enum");
        let variant = &enum_view.variants()[descriptor.variant.variant_id];
        if variant.payload.len() != descriptor.variant.arguments.len() {
            return Err(HIRError::invalid_funcall_arg_length(
                enum_view.name(),
                variant.payload.len(),
                descriptor.variant.arguments.len(),
                descriptor.span,
            ));
        }

        //Resolve any explicitly-provided generic arguments, e.g. the `[int, void]`
        //of `Result.Ok<int, void>(5)`, into their HIR type ids. These are the
        //concrete type arguments the enum reference should carry.
        let explicit = queue
            .get_node(self.file())
            .resolve_call_generics(descriptor.generics, &descriptor.context)?;

        //The number of generic parameters this enum declares, derived from the
        //highest generic-parameter index referenced by the variant's payload.
        let arity = crate::generics::implied_arity(&variant.payload, queue.hir);

        //The concrete type arguments of the reference, indexed by the enum's
        //type-parameter position. Explicit arguments win; anything still missing
        //is inferred from the payload argument's type.
        let mut generics = GenericTypeArguments::new();
        generics.reserve(arity);

        let mut arguments = Vec::new();
        for (arg_index, arg_type) in variant.payload.iter().enumerate() {
            let (expected_type, generic_index) = match queue.hir.view(*arg_type).raw() {
                HirType::GenericParam { index, .. } => {
                    let index = *index as usize;
                    let expected = explicit.get(index).copied().or_else(|| generics.get(index));
                    (expected, Some(index))
                }
                _ => (Some(*arg_type), None),
            };
            let expr = self.build_expression(
                queue,
                ExpressionDescriptor {
                    target: descriptor.variant.arguments[arg_index],
                    expected: expected_type,
                    context: &descriptor.context,
                },
            )?;
            arguments.push(expr);

            if let Some(index) = generic_index
                && explicit.get(index).is_none()
            {
                generics.set(index, queue.hir.view(expr.data).ty());
            }
        }

        //Back-fill any type parameters not mentioned by the built payload from
        //the explicit argument list, so the reference always carries the full,
        //ordered set of concrete type arguments.
        generics.merge_explicit(&explicit);

        let enum_ty = generics.finish_ref(queue.hir, descriptor.enum_type);
        Ok(HirExpression {
            ty: enum_ty,
            kind: HirExpressionKind::Enum {
                ty: enum_ty,
                variant: descriptor.variant.variant_id,
                args: arguments,
            },
        })
    }

    ///Builds an enum variant construction expression, e.g. `Some(4)` or `None`.
    pub(super) fn build_enum_variant_expression(
        &mut self,
        queue: &HirQueueBuilder,
        name: SymbolPointer,
        arguments: &[Spanned<DedupPoolId<ASTExpression>>],
        span: Span,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let (enum_ty, variant_index) = self
            .resolve_enum_variant(queue, name)
            .ok_or(HIRError::name_unrecognized(name, span))?;
        let enum_ty_view = queue.hir.view(enum_ty);
        let enum_type = enum_ty_view.dereference();
        let enum_view = enum_type
            .is_enum()
            .expect("resolve_enum_variant should always resolve an enum type");
        let variant = enum_view
            .variants()
            .get(variant_index)
            .ok_or(HIRError::variant_unrecognized(name, span))?;
        if variant.payload.len() != arguments.len() {
            return Err(HIRError::invalid_funcall_arg_length(
                name,
                variant.payload.len(),
                arguments.len(),
                span,
            ));
        }
        //The concrete type arguments of the reference, inferred from the
        //payload argument types. `enum_ty` holds the raw enum type; the
        //reference carries the concrete generic arguments so monomorphization
        //can later resolve it to its instantiated type.
        let mut generics = GenericTypeArguments::new();
        let args = arguments
            .iter()
            .zip(&variant.payload)
            .map(|(arg, ty)| {
                let expected = match queue.hir.view(*ty).raw() {
                    HirType::GenericParam { index, .. } => generics.get(*index as usize),
                    _ => Some(*ty),
                };
                let expr = self.build_expression(
                    queue,
                    ExpressionDescriptor {
                        target: *arg,
                        expected,
                        context,
                    },
                )?;
                if let HirType::GenericParam { index, .. } = queue.hir.view(*ty).raw() {
                    generics.set(*index as usize, queue.hir.view(expr.data).ty());
                }
                Ok(expr)
            })
            .collect::<Result<Vec<_>>>()?;
        let enum_ty = generics.finish_ref(queue.hir, enum_ty);
        Ok(HirExpression {
            ty: enum_ty,
            kind: HirExpressionKind::Enum {
                ty: enum_ty,
                variant: variant_index,
                args,
            },
        })
    }

    ///Builds a pattern-matching expression, `lhs matches Pattern`.
    ///
    /// The pattern may be a bare variant name (`None`) or a variant reference
    /// with a payload (`Some(4)`). A `matches` expression always produces a
    /// `bool`.
    pub(super) fn build_matches_expression(
        &mut self,
        queue: &HirQueueBuilder,
        value: Spanned<DedupPoolId<ASTExpression>>,
        pattern: Spanned<DedupPoolId<ASTExpression>>,
        span: Span,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let value = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: value,
                expected: None,
                context,
            },
        )?;

        let expr_view = queue.hir.view(value.data);
        let ty_viewer = expr_view.ty_viewer();
        let enum_type = ty_viewer.dereference();
        let enum_view = enum_type
            .is_enum()
            .ok_or_else(|| HIRError::matches_on_non_enum(enum_type.data, span))?;

        let (variant_name, pattern_args) = match queue.get_expr(pattern.data) {
            ASTExpression::Identifier(name) => (*name, &[][..]),
            ASTExpression::FunctionCall { name, args } => {
                let identifier = queue.get_plain_type(*name);
                (identifier.identifier, args.as_slice())
            }
            _ => return Err(HIRError::invalid_pattern(pattern.span)),
        };

        let variant_index = enum_view
            .find_variant(variant_name)
            .ok_or_else(|| HIRError::variant_unrecognized(variant_name, span))?;
        let variant = &enum_view.variants()[variant_index];
        if variant.payload.len() != pattern_args.len() {
            return Err(HIRError::invalid_funcall_arg_length(
                variant_name,
                variant.payload.len(),
                pattern_args.len(),
                span,
            ));
        }

        let args = pattern_args
            .iter()
            .zip(&variant.payload)
            .map(|(arg, ty)| {
                self.build_expression(
                    queue,
                    ExpressionDescriptor {
                        target: *arg,
                        expected: Some(*ty),
                        context,
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let ty = queue.hir.create_type(HirType::Bool);
        Ok(HirExpression {
            ty,
            kind: HirExpressionKind::Matches {
                value,
                variant: variant_index,
                args,
            },
        })
    }
}
