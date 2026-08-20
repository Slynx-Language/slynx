use std::ops::Deref;

use common::{Span, Spanned, pool::DedupPoolId};
use slynx_parser::{ASTExpression, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, HirType, Result, SymbolPointer,
    builders::HirQueueBuilder,
};

use super::ExpressionBuilder;

impl ExpressionBuilder {
    pub(super) fn build_deref(
        &mut self,
        queue: &HirQueueBuilder,
        expr: Spanned<DedupPoolId<ASTExpression>>,
        expected: Option<DedupPoolId<HirType>>,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let build_expr = self.build_expression(queue, expr, expected, context)?;
        let expr_ty = queue.hir.view(build_expr.data).ty();
        let expr_ty = match queue.hir.view(expr_ty).raw() {
            HirType::ImutableRef(inner) | HirType::MutableRef(inner) => *inner,
            _ => {
                return Err(HIRError::invalid_deref(expr.span));
            }
        };
        Ok(HirExpression {
            ty: expr_ty,
            kind: HirExpressionKind::Deref(build_expr),
        })
    }

    pub(super) fn build_reference(
        &mut self,
        queue: &HirQueueBuilder,
        expr: Spanned<DedupPoolId<ASTExpression>>,
        mutable: bool,
        expected: Option<DedupPoolId<HirType>>,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let expr = self.build_expression(queue, expr, expected, context)?;
        let expr_ty = queue.hir.view(expr.data).ty();
        let ty = queue.hir.create_type(if mutable {
            HirType::MutableRef(expr_ty)
        } else {
            HirType::ImutableRef(expr_ty)
        });
        Ok(HirExpression {
            ty,
            kind: HirExpressionKind::Reference(expr),
        })
    }

    pub(super) fn build_null(
        &self,
        queue: &HirQueueBuilder,
        span: Span,
        expected: Option<DedupPoolId<HirType>>,
    ) -> Result<HirExpression> {
        let ty = match expected {
            None => {
                return Err(HIRError::couldnt_infer(span));
            }
            Some(ty) if let HirType::Nullable(_) = queue.hir.deref()[ty] => ty,
            Some(ty) => {
                return Err(HIRError::unexpected_type(
                    ty,
                    queue.hir.create_type(HirType::Nullable(ty)),
                    span,
                ));
            }
        };
        Ok(HirExpression {
            ty,
            kind: HirExpressionKind::Null,
        })
    }

    pub(super) fn build_bool(&self, queue: &HirQueueBuilder, value: bool) -> HirExpression {
        HirExpression {
            ty: queue.hir.create_type(HirType::Bool),
            kind: if value {
                HirExpressionKind::True
            } else {
                HirExpressionKind::False
            },
        }
    }

    pub(super) fn build_int_literal(&self, queue: &HirQueueBuilder, value: i32) -> HirExpression {
        queue.hir.create_int_expression(value, 0)
    }

    pub(super) fn build_float_literal(&self, queue: &HirQueueBuilder, value: f32) -> HirExpression {
        queue.hir.create_float_expression(value)
    }

    pub(super) fn build_str_literal(
        &self,
        queue: &HirQueueBuilder,
        value: SymbolPointer,
    ) -> HirExpression {
        queue.hir.create_strliteral_expression(value)
    }
}
