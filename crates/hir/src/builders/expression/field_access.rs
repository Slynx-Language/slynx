use common::{
    Span, Spanned, VisibilityModifier,
    pool::{DedupPoolId, PoolId},
};
use either::Either;
use module_loader::{ASTType, FileId};
use slynx_parser::{ASTExpression, TypeContext};

use crate::{
    DescriptorId, HIRError, HirExpression, HirExpressionKind, Result, SymbolPointer,
    builders::{
        HirQueueBuilder,
        expression::{
            calls::{FunctionCallDescriptor, FunctionTarget},
            enums::{EnumExpressionDescriptor, EnumVariantDescriptor},
            fields_resolution::AccessParentCategory,
            literals::ReferenceExpressionDescriptor,
        },
        lowering::lowerer::LowerTypeDeclarationDescriptor,
    },
    fields_resolution::TypeAccessCategory,
    term::{TermId, TermNode},
};

use super::ExpressionBuilder;

///A descriptor for a field (or type member) access expression.
pub struct FieldAccessDescriptor<'a> {
    ///The parent we are accessing a member from. It can either be a raw AST
    ///expression that still needs to be built, or an already-built HIR
    ///expression (used when chaining accesses).
    pub parent: Either<Spanned<DedupPoolId<ASTExpression>>, Spanned<PoolId<HirExpression>>>,
    ///The field (or method) being accessed
    pub field: Spanned<DedupPoolId<ASTExpression>>,
    ///The span of the access expression, used for error reporting
    pub span: Span,
    ///The expected type of the access, if known
    pub expected: Option<TermId>,
    ///The type context used to resolve types
    pub context: &'a TypeContext<'a>,
}

impl ExpressionBuilder {
    ///Builds a member access on a parent expression. When the parent names a
    ///type, this resolves a static member access (such as a static method);
    ///otherwise it builds a value field access.
    pub(super) fn build_field_access(
        &mut self,
        queue: &HirQueueBuilder,
        descriptor: FieldAccessDescriptor<'_>,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        match self.resolve_access_field_category(queue, &descriptor)? {
            AccessParentCategory::TypeAccess { ast_type, span } => {
                self.build_type_access(queue, ast_type, descriptor.field, span, descriptor.context)
            }

            AccessParentCategory::NormalAccess(parent) => self.build_field_access_impl(
                queue,
                parent,
                descriptor.field,
                descriptor.span,
                descriptor.context,
            ),
        }
    }

    fn build_type_access(
        &mut self,
        queue: &HirQueueBuilder,
        ast_type: ASTType,
        child: Spanned<DedupPoolId<ASTExpression>>,
        span: Span,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        let lowered_type = queue.lowerer.lower_type_declaration(
            queue,
            LowerTypeDeclarationDescriptor {
                ast_type,
                context,
                span,
            },
        )?;

        let expr =
            match self.resolve_type_access_category(queue, child, lowered_type.clone(), span)? {
                TypeAccessCategory::Intermediate {
                    inner_parent,
                    inner_field,
                } => {
                    let parent =
                        self.build_type_access(queue, ast_type, inner_parent, span, context)?;
                    return self.build_field_access_impl(queue, parent, inner_field, span, context);
                }
                TypeAccessCategory::EnumVariant(variant_id) => self.build_enum_expression(
                    queue,
                    EnumExpressionDescriptor {
                        enum_type: lowered_type.term,
                        variant: EnumVariantDescriptor {
                            variant_id,
                            arguments: &[],
                        },
                        generics: &[],
                        span,
                        context,
                    },
                ),
                TypeAccessCategory::EnumPayloadVariant {
                    variant_id,
                    args: ref arguments,
                    payload_name,
                } => self.build_enum_expression(
                    queue,
                    EnumExpressionDescriptor {
                        enum_type: lowered_type.term,
                        variant: EnumVariantDescriptor {
                            variant_id,
                            arguments,
                        },
                        generics: &queue.get_plain_type(payload_name).generic,
                        span,
                        context,
                    },
                ),
                TypeAccessCategory::StaticMethod {
                    target,
                    name,
                    ref args,
                } => self.build_function_call(
                    queue,
                    FunctionCallDescriptor {
                        target: FunctionTarget::Resolved {
                            target,
                            type_arguments: &queue.get_plain_type(name).generic,
                        },
                        arguments: args,
                        span,
                        context,
                        prepended_arguments: &[],
                    },
                ),
            }?;

        Ok(span.make_spanned(queue.hir.store.insert_expression(expr)))
    }

    ///Parses a field access where the field being accessed is an identifier given by `field_name`. This is simply for expressions such as `a.b.c`, where the parent is `a.b`, and this is called with the field name `c` and `a.b` where the parent is `a` and this is called with field name `b`
    fn build_field_access_with_identifier(
        &mut self,
        queue: &HirQueueBuilder,
        parent: Spanned<PoolId<HirExpression>>,
        field_name: SymbolPointer,
        span: Span,
    ) -> Result<HirExpression> {
        let parent_ty = queue.hir[parent.data].ty;
        let parent_view = queue.hir.view(parent_ty);
        let concrete_type = parent_view.concrete_type();
        let dereferenced_type = parent_view.dereference();
        match dereferenced_type.is_struct() {
            Some(view)
                if let Some(position) = view.fields().iter().position(|f| f.data == field_name) =>
            {
                let field_ty = view.field_types()[position];
                let field_ty = match view.new_with(field_ty).raw().node() {
                    TermNode::Apply { args: generics, .. } => {
                        crate::generics::substitute_terms(queue.hir, generics, field_ty)
                    }
                    _ => field_ty,
                };
                Ok(HirExpression {
                    ty: field_ty,
                    kind: HirExpressionKind::FieldAccess {
                        expr: parent,
                        field_index: position,
                        field_name: Some(field_name),
                    },
                })
            }
            Some(_) => {
                //is struct but could not find any field
                return Err(HIRError::property_unrecognized(
                    dereferenced_type.data,
                    vec![field_name],
                    span,
                ));
            }
            None if parent_view.dereference().is_ref() //&T where T is a struct
                && let Some(view) = parent_view.concrete_type().is_struct() =>
            {
                let Some(position) = view.fields().iter().position(|f| f.data == field_name) else {
                    return Err(HIRError::property_unrecognized(
                        dereferenced_type.data,
                        vec![field_name],
                        span,
                    ));
                };
                let field_ty = view.field_types()[position];
                let field_ty = match queue.hir.view(parent_ty).raw().node() {
                    TermNode::Apply { args: generics, .. } => {
                        crate::generics::substitute_terms(queue.hir, generics, field_ty)
                    }
                    _ => field_ty,
                };

                let parent =
                    parent
                        .span
                        .make_spanned(queue.hir.store.insert_expression(HirExpression {
                            ty: concrete_type.data,
                            kind: HirExpressionKind::Deref(parent),
                        }));
                Ok(HirExpression {
                    ty: field_ty,
                    kind: HirExpressionKind::FieldAccess {
                        expr: parent,
                        field_index: position,
                        field_name: Some(field_name),
                    },
                })
            }
            None => {
                let ty = dereferenced_type.data;
                return Err(HIRError::not_a_struct(ty, span));
            }
        }
    }

    ///Builds a member access against an already-built parent expression.
    fn build_field_access_impl(
        &mut self,
        queue: &HirQueueBuilder,
        parent: Spanned<PoolId<HirExpression>>,
        field_ast: Spanned<DedupPoolId<ASTExpression>>,
        span: Span,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        let expr = match queue.get_expr(field_ast.data) {
            ASTExpression::FieldAccess {
                parent: inner_parent,
                field: inner_field,
            } => {
                let intermediate =
                    self.build_field_access_impl(queue, parent, *inner_parent, span, context)?;
                return self.build_field_access_impl(
                    queue,
                    intermediate,
                    *inner_field,
                    span,
                    context,
                );
            }
            ASTExpression::Identifier(field_name) => {
                self.build_field_access_with_identifier(queue, parent, *field_name, span)?
            }
            ASTExpression::FunctionCall { name, args } => {
                let name_sym = queue.type_name(name.data);
                let parent_type_view = queue.hir.view(queue.hir[parent.data].ty);
                let parent_ty = parent_type_view.dereference();

                let inherent = match parent_ty.is_struct() {
                    Some(view) => view
                        .method_named_as(name_sym, VisibilityModifier::Public)
                        .or_else(|| queue.hir.types.methods.method_of(parent_ty.data, name_sym)),
                    None => None,
                };
                let func_id = match inherent {
                    Some(id) => id,
                    None if let Some(id) =
                        queue.resolve_method(self.file(), parent_ty.data, name_sym, span)? =>
                    {
                        id
                    }
                    None => {
                        let concrete_self = {
                            let mut current = queue.hir[parent.data].ty;
                            loop {
                                match queue.hir.view(current).raw().node() {
                                    TermNode::Ref { target, .. } => current = *target,
                                    _ => break current,
                                }
                            }
                        };
                        if let Some(id) = queue.resolve_interface_method_for_concrete(
                            self.file(),
                            concrete_self,
                            name_sym,
                            span,
                        )? {
                            id
                        } else {
                            return Err(HIRError::missing_properties(vec![name_sym], span));
                        }
                    }
                };

                let prepend_args = {
                    let func_view = queue.hir.view(func_id);
                    let first_arg = match func_view.get_argument_type(0) {
                        Some(ty)
                            if let TermNode::Ref { mutable, .. } =
                                queue.hir.view(ty).raw().node() =>
                        {
                            self.build_reference_expression(
                                queue,
                                ReferenceExpressionDescriptor {
                                    target: Either::Right(parent),
                                    mutable: *mutable,
                                    context,
                                },
                            )?
                        }
                        _ => parent,
                    };
                    [first_arg]
                };
                self.build_function_call(
                    queue,
                    FunctionCallDescriptor {
                        target: FunctionTarget::Resolved {
                            target: func_id,
                            type_arguments: &queue.get_plain_type(*name).generic,
                        },
                        arguments: args,
                        prepended_arguments: &prepend_args,
                        span,
                        context,
                    },
                )?
            }
            _ => return Err(HIRError::invalid_field_access(span)),
        };
        Ok(span.make_spanned(queue.hir.store.insert_expression(expr)))
    }
}
