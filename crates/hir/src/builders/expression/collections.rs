use std::ops::Deref;

use common::{Span, Spanned, pool::DedupPoolId};
use slynx_parser::{ASTExpression, RangeType, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, HirType, Result, builders::HirQueueBuilder,
};

use super::{ExpressionBuilder, ExpressionDescriptor};

impl ExpressionBuilder {
    pub(super) fn build_tuple_expression(
        &mut self,
        queue: &HirQueueBuilder,
        fields: &[Spanned<DedupPoolId<ASTExpression>>],
        expected: Option<DedupPoolId<HirType>>,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let mut expressions = Vec::with_capacity(fields.len());
        let mut types = Vec::with_capacity(fields.len());

        for (idx, field) in fields.iter().enumerate() {
            let field_type = if let Some(expected) = expected
                && let Some(tuple) = queue.hir.view(expected).is_tuple()
            {
                Some(tuple.fields()[idx])
            } else {
                None
            };
            let expr = self.build_expression(
                queue,
                ExpressionDescriptor {
                    target: *field,
                    expected: field_type,
                    context,
                },
            )?;
            types.push(queue.hir[expr.data].ty);
            expressions.push(expr);
        }
        Ok(HirExpression {
            ty: queue.hir.create_tuple_type(types),
            kind: HirExpressionKind::Tuple(expressions),
        })
    }

    pub(super) fn build_tuple_access(
        &mut self,
        queue: &HirQueueBuilder,
        tuple: Spanned<DedupPoolId<ASTExpression>>,
        expected: Option<DedupPoolId<HirType>>,
        span: Span,
        index: usize,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let expr = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: tuple,
                expected,
                context,
            },
        )?;
        let raw_expr = &queue.hir[expr.data];
        let parent_view = queue.hir.view(raw_expr.ty);
        let resolved = parent_view.dereference();
        let ty = match resolved.is_tuple() {
            None => {
                let ty = resolved.data;
                return Err(HIRError::not_a_tuple(ty, span));
            }
            Some(tuple_view) => {
                let field_index = index;
                let fields = tuple_view.fields();
                if field_index >= fields.len() {
                    return Err(HIRError::invalid_tuple_index(
                        field_index,
                        fields.len(),
                        span,
                    ));
                }
                fields[field_index]
            }
        };
        Ok(HirExpression {
            ty,
            kind: HirExpressionKind::FieldAccess {
                expr,
                field_index: index,
                field_name: None,
            },
        })
    }

    pub(super) fn build_index(
        &mut self,
        queue: &HirQueueBuilder,
        expr: Spanned<DedupPoolId<ASTExpression>>,
        range: &RangeType,
        expected: Option<DedupPoolId<HirType>>,
        span: Span,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let expr = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: expr,
                expected,
                context,
            },
        )?;
        let after_index_type = {
            let expr_type = queue.hir[expr.data].ty;
            match &queue.hir.deref()[expr_type] {
                HirType::Vector(t) => *t,
                HirType::Array(t, _) => *t,
                HirType::GenericParam { .. } => expr_type,
                _ => return Err(HIRError::invalid_indexing(expr_type, span)),
            }
        };
        match range {
            RangeType::NoRange(index) => {
                let index = self.build_expression(
                    queue,
                    ExpressionDescriptor {
                        target: *index,
                        expected,
                        context,
                    },
                )?;
                let viewer = queue.hir.view(index.data);
                let ty_viewer = viewer.ty_viewer();
                match ty_viewer.raw() {
                    HirType::Int => {}
                    _ => {
                        return Err(HIRError::unexpected_type(
                            ty_viewer.data,
                            queue.hir.create_type(HirType::Int),
                            index.span,
                        ));
                    }
                }
                Ok(HirExpression {
                    kind: HirExpressionKind::ArrayIndex(expr, index),
                    ty: after_index_type,
                })
            }
            r => unimplemented!("Ranges {r:?} are not implemented yet"),
        }
    }

    pub(super) fn build_array(
        &mut self,
        queue: &HirQueueBuilder,
        expressions: &[Spanned<DedupPoolId<ASTExpression>>],
        span: Span,
        expected: Option<DedupPoolId<HirType>>,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let mut exprs = Vec::with_capacity(expressions.len());
        let Some(first) = expressions.first() else {
            return match expected {
                Some(ty) if queue.hir.view(ty).is_array().is_some() => Ok(HirExpression {
                    ty,
                    kind: HirExpressionKind::Array(exprs),
                }),
                Some(_) | None => Err(HIRError::couldnt_infer(span)),
            };
        };
        let (inner_type, size) = expected.and_then(|e| queue.hir.view(e).is_array()).unzip();
        let expr = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: *first,
                expected: inner_type,
                context,
            },
        )?;
        let ty = queue.hir[expr.data].ty;
        if let Some(expected) = inner_type {
            self.unify_types(queue, ty, expected, span)?;
        }
        exprs.push(expr);
        for expr in &expressions[1..] {
            let expr = self.build_expression(
                queue,
                ExpressionDescriptor {
                    target: *expr,
                    expected: Some(ty),
                    context,
                },
            )?;
            exprs.push(expr);
        }
        let final_length = size.unwrap_or(exprs.len());
        if let Some(expected_len) = size
            && final_length != expected_len
        {
            return Err(HIRError::array_length_mismatch(
                expected_len,
                final_length,
                span,
            ));
        }
        let final_type = queue.hir.create_type(HirType::Array(ty, final_length));
        Ok(HirExpression {
            ty: final_type,
            kind: HirExpressionKind::Array(exprs),
        })
    }

    pub(super) fn build_vector(
        &mut self,
        queue: &HirQueueBuilder,
        expressions: &[Spanned<DedupPoolId<ASTExpression>>],
        span: Span,
        expected: Option<DedupPoolId<HirType>>,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let mut exprs = Vec::with_capacity(expressions.len());
        let Some(first) = expressions.first() else {
            return match expected {
                Some(ty) if queue.hir.view(ty).is_vector().is_some() => Ok(HirExpression {
                    ty,
                    kind: HirExpressionKind::Vector(exprs),
                }),
                Some(_) | None => Err(HIRError::couldnt_infer(span)),
            };
        };
        let inner_type = expected.and_then(|e| queue.hir.view(e).is_vector());
        let expr = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: *first,
                expected: inner_type,
                context,
            },
        )?;
        let ty = queue.hir[expr.data].ty;
        if let Some(expected) = inner_type {
            self.unify_types(queue, ty, expected, span)?;
        }
        exprs.push(expr);
        for expr in &expressions[1..] {
            let expr = self.build_expression(
                queue,
                ExpressionDescriptor {
                    target: *expr,
                    expected: Some(ty),
                    context,
                },
            )?;
            exprs.push(expr);
        }
        let final_type = queue.hir.create_type(HirType::Vector(ty));
        Ok(HirExpression {
            ty: final_type,
            kind: HirExpressionKind::Vector(exprs),
        })
    }
}
