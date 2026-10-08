use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use either::Either;
use module_loader::ASTType;
use slynx_parser::{ASTExpression, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, Result, SymbolPointer,
    builders::{
        HirQueueBuilder,
        expression::{
            calls::{FunctionCallDescriptor, FunctionTarget},
            enums::{EnumExpressionDescriptor, EnumVariantDescriptor},
            literals::ReferenceExpressionDescriptor,
        },
        lowering::lowerer::LowerTypeDeclarationDescriptor,
    },
    resolution::fields::{AccessParentCategory, FieldAccessCategory, TypeAccessCategory},
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
        let lowered_type = queue.lowerer.materialize_type_declaration(
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
        let dereferenced_type = parent_view.nominal();
        match () {
            _ if let Some(view) = dereferenced_type.is_struct()
                && let Some(position) = view.fields().iter().position(|f| f.name == field_name) =>
            {
                let field_ty = view.fields()[position].data.ty;
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
            _ if dereferenced_type.is_struct().is_some() => {
                //is struct but could not find any field
                return Err(HIRError::property_unrecognized(
                    dereferenced_type.data,
                    vec![field_name],
                    span,
                ));
            }
            _ if dereferenced_type.is_ref()
                && let Some(view) = concrete_type.is_struct() =>
            {
                let Some(position) = view.fields().iter().position(|f| f.name == field_name) else {
                    return Err(HIRError::property_unrecognized(
                        dereferenced_type.data,
                        vec![field_name],
                        span,
                    ));
                };
                let field_ty = view.fields()[position].ty;
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
            _ => {
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
        let expr = match self.resolve_field_access_category(queue, parent, field_ast, span)? {
            FieldAccessCategory::Field(field_name) => {
                self.build_field_access_with_identifier(queue, parent, field_name, span)?
            }
            FieldAccessCategory::Intermediate {
                inner_parent,
                inner_field,
            } => {
                let intermediate =
                    self.build_field_access_impl(queue, parent, inner_parent, span, context)?;
                return self.build_field_access_impl(
                    queue,
                    intermediate,
                    inner_field,
                    span,
                    context,
                );
            }
            FieldAccessCategory::Method {
                target,
                name,
                ref args,
            } => {
                let prepend_args = vec![if let Some(ty) =
                    queue.hir.view(target).get_argument_type(0)
                    && let TermNode::Ref { mutable, .. } = queue.hir.view(ty).raw().node()
                {
                    self.build_reference_expression(
                        queue,
                        ReferenceExpressionDescriptor {
                            target: Either::Right(parent),
                            mutable: *mutable,
                            context,
                        },
                    )?
                } else {
                    parent
                }];
                self.build_function_call(
                    queue,
                    FunctionCallDescriptor {
                        target: FunctionTarget::Resolved {
                            target: target,
                            type_arguments: &queue.get_plain_type(name).generic,
                        },
                        arguments: args,
                        prepended_arguments: &prepend_args,
                        span,
                        context,
                    },
                )?
            }
        };

        Ok(span.make_spanned(queue.hir.store.insert_expression(expr)))
    }
}
