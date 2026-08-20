use common::{Span, Spanned, pool::DedupPoolId};
use slynx_parser::{ASTExpression, ASTStatement, TypeContext};

use crate::{
    HirExpression, HirExpressionKind, HirStatement, HirType, Result, builders::HirQueueBuilder,
};

use super::ExpressionBuilder;

impl ExpressionBuilder {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn build_if(
        &mut self,
        queue: &HirQueueBuilder,
        condition: Spanned<DedupPoolId<ASTExpression>>,
        body: &[Spanned<DedupPoolId<ASTStatement>>],
        else_body: &[Spanned<DedupPoolId<ASTStatement>>],
        span: Span,
        expected: Option<DedupPoolId<HirType>>,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let condition = self.build_expression(queue, condition, expected, context)?;
        let bool_ty = queue.hir.create_type(HirType::Bool);
        self.unify_types(queue, queue.hir[condition.data].ty, bool_ty, span)?;

        let then_branch = body
            .iter()
            .map(|stmt| self.build_statement(queue, stmt, context))
            .collect::<Result<Vec<_>>>()?;
        let else_branch = if else_body.is_empty() {
            None
        } else {
            Some(
                else_body
                    .iter()
                    .map(|stmt| self.build_statement(queue, stmt, context))
                    .collect::<Result<Vec<_>>>()?,
            )
        };

        let then_ty = then_branch
            .last()
            .map(|s| match &queue.hir[s.data] {
                HirStatement::Expression { expr } => queue.hir[expr.data].ty,
                HirStatement::Variable { value, .. } => queue.hir[value.data].ty,
                _ => queue.hir.create_type(HirType::Void),
            })
            .unwrap_or_else(|| queue.hir.create_type(HirType::Void));
        let else_ty = else_branch
            .as_ref()
            .and_then(|b| b.last())
            .map(|s| match &queue.hir[s.data] {
                HirStatement::Expression { expr } => queue.hir[expr.data].ty,
                HirStatement::Variable { value, .. } => queue.hir[value.data].ty,
                _ => queue.hir.create_type(HirType::Void),
            })
            .unwrap_or_else(|| queue.hir.create_type(HirType::Void));
        self.unify_types(queue, else_ty, then_ty, span)?;
        Ok(HirExpression {
            ty: then_ty,
            kind: HirExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            },
        })
    }
}
