use common::{Span, Spanned, pool::DedupPoolId};
use slynx_parser::{ASTExpression, ASTStatement, TypeContext};

use crate::{
    HirExpression, HirExpressionKind, HirStatement, HirType, Result, builders::HirQueueBuilder,
};

use super::{ExpressionBuilder, ExpressionDescriptor};

///A descriptor for an if expression.
pub struct IfExpressionDescriptor<'a> {
    ///The condition of the if expression
    pub condition: Spanned<DedupPoolId<ASTExpression>>,
    ///The statements of the then branch
    pub body: &'a [Spanned<DedupPoolId<ASTStatement>>],
    ///The statements of the else branch, if any
    pub else_body: &'a [Spanned<DedupPoolId<ASTStatement>>],
    ///The span of the if expression, used for error reporting
    pub span: Span,
    ///The expected type of the if expression, if known
    pub expected: Option<DedupPoolId<HirType>>,
    ///The type context used to resolve types
    pub context: &'a TypeContext<'a>,
}

impl ExpressionBuilder {
    pub(super) fn build_if(
        &mut self,
        queue: &HirQueueBuilder,
        descriptor: IfExpressionDescriptor<'_>,
    ) -> Result<HirExpression> {
        let IfExpressionDescriptor {
            condition,
            body,
            else_body,
            span,
            expected,
            context,
        } = descriptor;
        let condition = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: condition,
                expected,
                context,
            },
        )?;
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
