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
        expression::{calls::FunctionCallDescriptor, literals::ReferenceExpressionDescriptor},
    },
    helpers::Visible,
};

use super::ExpressionBuilder;

impl ExpressionBuilder {
    pub(super) fn build_access(
        &mut self,
        queue: &HirQueueBuilder,
        parent: Spanned<DedupPoolId<ASTExpression>>,
        field_ast: Spanned<DedupPoolId<ASTExpression>>,
        expected: Option<DedupPoolId<HirType>>,
        span: Span,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        match queue.get_expr(parent.data) {
            ASTExpression::Identifier(ident)
                if let Ok((file, ty)) = queue
                    .get_node(self.file())
                    .find_type_named_as(parent.span.make_spanned(*ident), context) =>
            {
                self.build_type_access(queue, file, ty, field_ast, span, context)
            }
            _ => {
                let parent = self.build_expression(queue, parent, expected, context)?;
                self.build_field_access(queue, parent, field_ast, span, context)
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
                self.build_field_access(queue, parent, *inner_field, span, context)
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
                    let call = self.build_call_for(
                        queue,
                        child.span.make_spanned(FunctionCallDescriptor {
                            target: method,
                            received_args: args,
                            prepend_arg: &[],
                            received_generics: generics,
                            context,
                        }),
                    )?;

                    Ok(span.make_spanned(queue.hir.insert_expression(call)))
                } else {
                    Err(HIRError::static_method_not_found(method_name, span))
                }
            }
            _ => Err(HIRError::invalid_type_access(span)),
        }
    }

    pub(super) fn build_field_access(
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
                    self.build_field_access(queue, parent, *inner_parent, span, context)?;
                return self.build_field_access(queue, intermediate, *inner_field, span, context);
            }
            ASTExpression::Identifier(field_name) => {
                let parent_ty = queue.hir[parent.data].ty;
                let parent_view = queue.hir.view(parent_ty);
                let resolved = parent_view.concrete_type();
                match resolved.is_struct() {
                    Some(view) => {
                        let (fields, field_types) = (view.fields(), view.field_types());
                        let position = fields
                            .iter()
                            .position(|f| f.data == *field_name)
                            .ok_or_else(|| {
                                HIRError::property_unrecognized(
                                    resolved.data,
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
                    None if let Some(refr) = resolved.is_mutable_ref()
                        && let Some(view) = refr.dereference().is_struct() =>
                    {
                        let (fields, field_types) = (view.fields(), view.field_types());
                        let position = fields
                            .iter()
                            .position(|f| f.data == *field_name)
                            .ok_or_else(|| {
                                HIRError::property_unrecognized(
                                    resolved.data,
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
                                    ty: refr.data,
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
                        let ty = resolved.data;
                        return Err(HIRError::not_a_struct(ty, span));
                    }
                }
            }
            ASTExpression::FunctionCall { name, args } => {
                let name_sym = queue.type_name(name.data, &TypeContext::EMPTY);
                let parent_type_view = queue.hir.view(queue.hir[parent.data].ty);
                let parent_ty = parent_type_view.concrete_type();
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
                        let call = self.build_call_for(
                            queue,
                            span.make_spanned(FunctionCallDescriptor {
                                target: func_id,
                                received_args: args,
                                prepend_arg: &prepend_args,
                                received_generics: &queue.get_plain_type(*name).generic,
                                context,
                            }),
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
