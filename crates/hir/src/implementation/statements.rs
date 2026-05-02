use crate::{
    Result, SlynxHir,
    model::{HirStatement, HirStatementKind},
};
use common::ast::{ASTExpression, ASTExpressionKind, ASTStatement, ASTStatementKind};

impl SlynxHir {
    /// Checks that the given expression refers to an already-defined name, returning an error if not.
    pub fn check_existance(&mut self, expr: &ASTExpression) -> Result<()> {
        match &expr.kind {
            ASTExpressionKind::FieldAccess { parent, .. } => {
                self.check_existance(parent)?;
            }
            ASTExpressionKind::TupleAccess { tuple, .. } => {
                self.check_existance(tuple)?;
            }
            ASTExpressionKind::Identifier(name) => {
                let name = self.modules.intern_name(name);
                self.get_variable(name, &expr.span)?;
            }
            _ => {}
        }
        Ok(())
    }
    /// Resolves an AST statement into a typed [`HirStatement`].
    pub fn resolve_statement(&mut self, statement: ASTStatement) -> Result<HirStatement> {
        match statement.kind {
            ASTStatementKind::Expression(expr) => {
                let expr = self.resolve_expr(expr, None)?;
                Ok(HirStatement::new_expression(expr))
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
                let name = self.modules.intern_name(&name);
                let id = self.create_mutable_variable(name, rhs.ty, &statement.span)?;

                Ok(HirStatement::new_variable(id, rhs, statement.span))
            }
            ASTStatementKind::Var { name, ty, rhs } => {
                let typeid = ty.and_then(|t| {
                    self.retrieve_information_of_type(&t.identifier, &statement.span)
                        .ok()
                        .map(|inner| inner.0)
                });
                let rhs = self.resolve_expr(rhs, typeid)?;
                let name = self.modules.intern_name(&name);
                let id = self.create_variable(name, rhs.ty, &statement.span)?;

                Ok(HirStatement::new_variable(id, rhs, statement.span))
            }

            ASTStatementKind::While { condition, body } => {
                let condition = self.resolve_expr(condition, None)?;
                let body = body
                    .into_iter()
                    .map(|stmt| self.resolve_statement(stmt))
                    .collect::<Result<Vec<_>>>()?;
                Ok(HirStatement::new_while(condition, body, statement.span))
            }
        }
    }
}
