use color_eyre::eyre::Result;

use crate::hir::{
    SlynxHir,
    definitions::{HirStatement, HirStatementKind},
};
use common::ast::{ASTExpression, ASTExpressionKind, ASTStatement, ASTStatementKind};

impl SlynxHir {
    pub fn check_existance(&mut self, expr: &ASTExpression) -> Result<()> {
        match &expr.kind {
            ASTExpressionKind::FieldAccess { parent, .. } => {
                self.check_existance(parent)?;
            }
            ASTExpressionKind::Identifier(name) => {
                self.get_variable(name, &expr.span)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn resolve_statement(&mut self, statement: ASTStatement) -> Result<HirStatement> {
        match statement.kind {
            ASTStatementKind::Expression(expr) => {
                let expr = self.resolve_expr(expr, None)?;
                Ok(HirStatement {
                    span: expr.span.clone(),
                    kind: HirStatementKind::Expression { expr },
                })
            }
            ASTStatementKind::Assign { lhs, rhs } => {
                let lhs = self.resolve_expr(lhs, None)?;
                let rhs = self.resolve_expr(rhs, None)?;

                Ok(HirStatement {
                    kind: HirStatementKind::Assign { lhs, value: rhs },
                    span: statement.span,
                })
            }
            ASTStatementKind::MutableVar { name, ty, rhs } => {
                let typeid = ty.and_then(|t| {
                    self.retrieve_information_of_type(&t.identifier, &statement.span)
                        .ok()
                        .map(|inner| inner.0)
                });
                let rhs = self.resolve_expr(rhs, typeid)?;

                let id = self.create_variable(&name, rhs.ty, true);

                Ok(HirStatement {
                    kind: HirStatementKind::Variable {
                        name: id,
                        value: rhs,
                    },
                    span: statement.span,
                })
            }
            ASTStatementKind::Var { name, ty, rhs } => {
                let typeid = ty.and_then(|t| {
                    self.retrieve_information_of_type(&t.identifier, &statement.span)
                        .ok()
                        .map(|inner| inner.0)
                });
                let rhs = self.resolve_expr(rhs, typeid)?;

                let id = self.create_variable(&name, rhs.ty, false);

                Ok(HirStatement {
                    kind: HirStatementKind::Variable {
                        name: id,
                        value: rhs,
                    },
                    span: statement.span,
                })
            }
        }
    }
}
