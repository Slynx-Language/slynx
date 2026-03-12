use color_eyre::eyre::Result;

use crate::checker::TypeChecker;
use crate::hir::{
    TypeId,
    definitions::{HirStatement, HirStatementKind},
};

impl TypeChecker {
    pub(super) fn default_statement(
        &mut self,
        statement: &mut HirStatement,
        expected: &TypeId,
    ) -> Result<()> {
        match &mut statement.kind {
            HirStatementKind::Variable { value, .. } => {
                value.ty = self.resolve(&value.ty, &statement.span)?;
            }
            HirStatementKind::Assign { lhs, value } => {
                let ty = self.resolve(&lhs.ty, &lhs.span)?;
                value.ty = self.unify(&ty, &value.ty, &value.span)?;
            }
            HirStatementKind::Expression { expr } => self.default_expr(expr)?,
            HirStatementKind::Return { expr } => {
                self.default_expr(expr)?;
                let unify = self.unify(&expr.ty, expected, &statement.span)?;
                expr.ty = unify;
            }
            HirStatementKind::If {
                condition,
                body,
                else_body,
            } => {
                self.default_expr(condition)?;
                let unify =
                    self.unify(&condition.ty, &self.types_module.bool_id(), &condition.span)?;
                condition.ty = unify;
                for s in body {
                    self.default_statement(s, expected)?;
                }
                if let Some(else_body) = else_body {
                    for s in else_body {
                        self.default_statement(s, expected)?;
                    }
                }
            }
        };
        Ok(())
    }
}
