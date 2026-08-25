use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use slynx_parser::{ASTStatement, TypeContext};

use crate::{HirStatement, HirType, Result, builders::HirQueueBuilder};

use super::ExpressionBuilder;

impl ExpressionBuilder {
    pub(crate) fn build_statement(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        statement: &Spanned<DedupPoolId<ASTStatement>>,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirStatement>>> {
        let (data, span) = self.build_statement_data(queue, statement, context)?;
        let id = queue.hir.insert_statement(data);
        Ok(span.make_spanned(id))
    }

    /// Builds a statement and returns the raw `HirStatement` without inserting
    /// into the pool. Used for the last statement in function bodies where we
    /// may need to wrap it in an implicit return.
    pub(crate) fn build_statement_data(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        statement: &Spanned<DedupPoolId<ASTStatement>>,
        context: &TypeContext,
    ) -> Result<(HirStatement, Span)> {
        let stmt = queue.get_statement(statement.data);
        let data = match stmt {
            ASTStatement::Expression(e) => {
                let expr = self.build_expression(queue, *e, None, context)?;

                HirStatement::Expression { expr }
            }
            ASTStatement::Var { name, ty, rhs } | ASTStatement::MutableVar { name, ty, rhs } => {
                let var_type = if let Some(ty) = ty {
                    Some(queue.get_node(self.file()).find_type(*ty, context)?.1)
                } else {
                    None
                };
                let expr = self.build_expression(queue, *rhs, var_type, context)?;
                let exprty = queue.hir.view(expr.data).ty();
                let canmove = queue.hir.view(expr.data).can_move();
                let expected_type = if let Some(expected_ty) = ty {
                    queue
                        .get_node(self.file())
                        .find_type(*expected_ty, context)?
                        .1
                } else {
                    exprty
                };
                let ty = self.unify_types(queue, exprty, expected_type, statement.span)?;
                let varid = self.create_variable(
                    *name,
                    matches!(stmt, ASTStatement::MutableVar { .. }),
                    ty,
                );
                if let Some(variable) = canmove {
                    let varname = self.variable_name(varid);
                    self.borrowing_mut(variable).mark_moved(varname);
                }

                HirStatement::Variable {
                    name: varid,
                    value: expr,
                }
            }
            ASTStatement::Assign { lhs, rhs } => {
                let lhs = self.build_expression(queue, *lhs, None, context)?;
                self.is_expression_able_to_write(queue, lhs)?;
                let rhs = self.build_expression(queue, *rhs, None, context)?;
                self.unify_types(
                    queue,
                    queue.hir.view(rhs.data).ty(),
                    queue.hir.view(lhs.data).ty(),
                    statement.span,
                )?;
                HirStatement::Assign { lhs, value: rhs }
            }
            ASTStatement::While { condition, body } => {
                let condition = self.build_expression(
                    queue,
                    *condition,
                    Some(queue.hir.create_type(HirType::Bool)),
                    context,
                )?;
                let body = body
                    .iter()
                    .map(|statement| self.build_statement(queue, statement, context))
                    .collect::<Result<_>>()?;

                HirStatement::While { condition, body }
            }
            ASTStatement::Return { value } => {
                let expr = value
                    .map(|v| self.build_expression(queue, v, None, context))
                    .transpose()?;
                HirStatement::Return { expr }
            }
        };
        Ok((data, statement.span))
    }
}
