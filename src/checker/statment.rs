use color_eyre::eyre::Result;

use crate::{
    checker::TypeChecker,
    hir::{
        TypeId,
        deffinitions::{HirStatment, HirStatmentKind},
    },
};

impl TypeChecker {
    pub(super) fn default_statment(
        &mut self,
        statment: &mut HirStatment,
        expected: &TypeId,
    ) -> Result<()> {
        match &mut statment.kind {
            HirStatmentKind::Variable { value, .. } => {
                value.ty = self.resolve(&value.ty, &statment.span)?;
            }
            HirStatmentKind::Assign { lhs, value } => {
                let ty = self.resolve(&lhs.ty, &lhs.span)?;
                value.ty = self.unify(&ty, &value.ty, &value.span)?;
            }
            HirStatmentKind::Expression { expr } => self.default_expr(expr)?,
            HirStatmentKind::Return { expr } => {
                self.default_expr(expr)?;
                let unify = self.unify(&expr.ty, expected, &statment.span)?;
                expr.ty = unify;
            }
        };
        Ok(())
    }
}
