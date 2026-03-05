use color_eyre::eyre::Result;

use crate::{
    checker::TypeChecker,
    
};
use middleend::hir::{
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
        };
        Ok(())
    }
}
