use common::{
    Span, Spanned, VisibilityModifier,
    pool::{DedupPoolId, PoolId},
};
use either::Either;
use module_loader::FileId;
use slynx_parser::{ASTExpression, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, HirType, Result,
    builders::{
        HirQueueBuilder,
        expression::{
            calls::{FunctionCallDescriptor, FunctionTarget},
            literals::ReferenceExpressionDescriptor,
        },
    },
    helpers::Visible,
};

use super::{ExpressionBuilder, ExpressionDescriptor};

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
    pub expected: Option<DedupPoolId<HirType>>,
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
        let FieldAccessDescriptor {
            parent,
            field,
            span,
            expected,
            context,
        } = descriptor;
        match parent {
            Either::Left(parent) => match queue.get_expr(parent.data) {
                ASTExpression::Identifier(ident)
                    if let Ok((file, ty)) = queue
                        .get_node(self.file())
                        .find_type_named_as(parent.span.make_spanned(*ident), context) =>
                {
                    self.build_type_access(queue, file, ty, field, span, context)
                }
                _ => {
                    let parent = self.build_expression(
                        queue,
                        ExpressionDescriptor {
                            target: parent,
                            expected,
                            context,
                        },
                    )?;
                    self.build_field_access_impl(queue, parent, field, span, context)
                }
            },
            Either::Right(parent) => {
                self.build_field_access_impl(queue, parent, field, span, context)
            }
        }
    }

    fn build_type_access(
        &mut self,
        queue: &HirQueueBuilder,
        file_owner: FileId,
        ty: DedupPoolId<HirType>,
        child: Spanned<DedupPoolId<ASTExpression>>,
        span: Span,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        match queue.get_expr(child.data) {
            ASTExpression::FieldAccess {
                parent: inner_parent,
                field: inner_field,
            } => {
                let parent =
                    self.build_type_access(queue, file_owner, ty, *inner_parent, span, context)?;
                self.build_field_access_impl(queue, parent, *inner_field, span, context)
            }
            ASTExpression::Identifier(ident) => {
                unimplemented!("Constant values bound to types are not supported yet")
            }
            ASTExpression::FunctionCall { name, args } => {
                let method_name = queue.type_name(name.data, &TypeContext::EMPTY);
                if let Some(method) = queue.resolve_method(file_owner, ty, method_name, span)? {
                    let generics = {
                        let plain = queue.get_plain_type(*name);
                        &plain.generic
                    };
                    let call = self.build_function_call(
                        queue,
                        FunctionCallDescriptor {
                            target: FunctionTarget::Resolved {
                                target: method,
                                type_arguments: generics,
                            },
                            arguments: args,
                            prepended_arguments: &[],
                            span,
                            context,
                        },
                    )?;

                    Ok(span.make_spanned(queue.hir.insert_expression(call)))
                } else {
                    Err(HIRError::static_method_not_found(method_name, span))
                }
            }
            _ => Err(HIRError::invalid_type_access(span)),
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
                let parent_ty = queue.hir[parent.data].ty;
                let parent_view = queue.hir.view(parent_ty);
                let concrete_type = parent_view.concrete_type();
                let dereferenced_type = parent_view.dereference();
                match dereferenced_type.is_struct() {
                    Some(view) => {
                        let (fields, field_types) = (view.fields(), view.field_types());
                        let position = fields
                            .iter()
                            .position(|f| f.data == *field_name)
                            .ok_or_else(|| {
                                HIRError::property_unrecognized(
                                    dereferenced_type.data,
                                    vec![*field_name],
                                    span,
                                )
                            })?;

                        let field_ty = field_types[position];
                        let field_ty = match queue.hir.view(parent_ty).raw() {
                            HirType::Reference { generics, .. } => {
                                queue.substitute_generics(generics, field_ty)
                            }
                            _ => field_ty,
                        };
                        HirExpression {
                            ty: field_ty,
                            kind: HirExpressionKind::FieldAccess {
                                expr: parent,
                                field_index: position,
                                field_name: Some(*field_name),
                            },
                        }
                    }
                    None if parent_view.dereference().is_ref()
                        && let Some(view) = parent_view.concrete_type().is_struct() =>
                    {
                        let (fields, field_types) = (view.fields(), view.field_types());
                        let position = fields
                            .iter()
                            .position(|f| f.data == *field_name)
                            .ok_or_else(|| {
                                HIRError::property_unrecognized(
                                    concrete_type.data,
                                    vec![*field_name],
                                    span,
                                )
                            })?;

                        let field_ty = field_types[position];
                        let field_ty = match queue.hir.view(parent_ty).raw() {
                            HirType::Reference { generics, .. } => {
                                queue.substitute_generics(generics, field_ty)
                            }
                            _ => field_ty,
                        };

                        let parent =
                            parent
                                .span
                                .make_spanned(queue.hir.insert_expression(HirExpression {
                                    ty: concrete_type.data,
                                    kind: HirExpressionKind::Deref(parent),
                                }));
                        HirExpression {
                            ty: field_ty,
                            kind: HirExpressionKind::FieldAccess {
                                expr: parent,
                                field_index: position,
                                field_name: Some(*field_name),
                            },
                        }
                    }
                    None => {
                        let ty = dereferenced_type.data;
                        return Err(HIRError::not_a_struct(ty, span));
                    }
                }
            }
            ASTExpression::FunctionCall { name, args } => {
                let name_sym = queue.type_name(name.data, &TypeContext::EMPTY);
                let parent_type_view = queue.hir.view(queue.hir[parent.data].ty);
                let parent_ty = parent_type_view.dereference();
                match parent_ty.is_struct() {
                    None => return Err(HIRError::not_a_struct(parent_ty.data, span)),
                    Some(view) => {
                        let func_id = if let Some(method) =
                            view.method_named_as(name_sym, VisibilityModifier::Public)
                        {
                            Some(method)
                        } else {
                            queue
                                .hir
                                .methods
                                .get(&parent_ty.data)
                                .and_then(|methods| methods.get(&name_sym).map(|v| *v.value()))
                        };

                        let func_id = match func_id {
                            Some(id) => id,
                            None if let Some(id) = queue.resolve_method(
                                self.file(),
                                parent_ty.data,
                                name_sym,
                                span,
                            )? =>
                            {
                                id
                            }
                            _ => {
                                return Err(HIRError::missing_properties(vec![name_sym], span));
                            }
                        };

                        let prepend_args = {
                            let func_view = queue.hir.view(func_id);
                            let first_arg = match func_view.get_argument_type(0) {
                                Some(ty)
                                    if let HirType::ImutableRef(_) | HirType::MutableRef(_) =
                                        queue.hir.view(ty).raw() =>
                                {
                                    let mutable =
                                        matches!(queue.hir.view(ty).raw(), HirType::MutableRef(_));
                                    let reference = self.build_reference_expression(
                                        queue,
                                        ReferenceExpressionDescriptor {
                                            target: Either::Right(parent),
                                            mutable: mutable,
                                            context,
                                        },
                                    )?;
                                    reference
                                }
                                _ => parent,
                            };
                            [first_arg]
                        };
                        let call = self.build_function_call(
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
                        )?;
                        call
                    }
                }
            }
            _ => return Err(HIRError::invalid_field_access(span)),
        };
        Ok(span.make_spanned(queue.hir.insert_expression(expr)))
    }
}
