use common::{
    pool::{DedupPoolId, PoolId},
    Span, Spanned, VisibilityModifier,
};
use module_loader::FileId;
use slynx_parser::{ASTExpression, TypeContext};

use crate::{
    builders::HirQueueBuilder, helpers::Visible, HIRError, HirExpression, HirExpressionKind,
    HirType, Result,
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
                let type_name = queue.type_name(name.data, &TypeContext::EMPTY);
                if let Some(method) = queue.resolve_method(file_owner, ty, type_name)? {
                    let call = self.build_function_call(queue, *name, args, context)?;
                    Ok(span.make_spanned(queue.hir.insert_expression(call)))
                } else {
                    Err(HIRError::static_method_not_found(type_name, span))
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
                let resolved = parent_view.dereference();
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
                let parent_ty = queue.hir[parent.data].ty;
                let real_ty = queue.hir.view(parent_ty);
                let deref = real_ty.dereference();
                match deref.is_struct() {
                    None => return Err(HIRError::not_a_struct(deref.data, span)),
                    Some(view) => {
                        let func_id =
                            view.methods()
                                .iter()
                                .find_map(
                                    |Visible {
                                         data: (method, func),
                                         visibility,
                                     }| {
                                        (*method == name_sym
                                            && *visibility == VisibilityModifier::Public)
                                            .then_some(*func)
                                    },
                                )
                                .or_else(|| {
                                    queue.hir.methods.get(&deref.data).and_then(|methods| {
                                        methods.get(&name_sym).map(|v| *v.value())
                                    })
                                });

                        let func_id = match func_id {
                            Some(id) => id,
                            None if let Some(id) =
                                queue.resolve_method(self.file(), deref.data, name_sym)? =>
                            {
                                id
                            }
                            _ => {
                                return Err(HIRError::missing_properties(vec![name_sym], span));
                            }
                        };

                        let func_view = queue.hir.view(func_id);
                        let expected = func_view
                            .ty()
                            .is_function()
                            .expect("Method should be a function")
                            .arguments()
                            .to_vec();
                        if expected.len() != args.len() + 1 {
                            //+1 due to being a method call, so self is implicit
                            return Err(HIRError::invalid_funcall_arg_length(
                                name_sym,
                                expected.len(),
                                args.len(),
                                name.span,
                            ));
                        }
                        let built_args = args
                            .iter()
                            .enumerate()
                            .map(|(idx, arg)| {
                                self.build_expression(queue, *arg, Some(expected[idx]), context)
                            })
                            .collect::<Result<Vec<_>>>()?;
                        let mut method_args = vec![parent];
                        method_args.extend(built_args);
                        HirExpression {
                            ty: func_view.return_type(),
                            kind: HirExpressionKind::FunctionCall {
                                name: func_id,
                                args: method_args,
                                generics: Vec::new(),
                            },
                        }
                    }
                }
            }
            _ => return Err(HIRError::invalid_field_access(span)),
        };
        Ok(span.make_spanned(queue.hir.insert_expression(expr)))
    }
}
