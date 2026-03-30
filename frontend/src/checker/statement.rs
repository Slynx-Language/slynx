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
            HirStatementKind::Variable { name, value } => {
                // Ensure the initializer expression is fully typed and propagate it to the variable.
                self.default_expr(value)?;
                let ty = self.get_type_of_expr(value)?;
                value.ty = ty;
                self.types_module.insert_variable(*name, ty);
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

            HirStatementKind::While { condition, body } => {
                condition.ty = self.get_type_of_expr(condition)?;
                let bool_id = self.types_module.bool_id();
                condition.ty = self.unify(&condition.ty, &bool_id, &condition.span)?;
                for stmt in body {
                    self.default_statement(stmt, expected)?;
                }
            }
        };

        Ok(())
    }
}
