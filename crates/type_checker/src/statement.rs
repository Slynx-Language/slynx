//! Type-checking logic for statements in the Slynx compiler.
//!
//! This module provides the implementation for type-checking statements, including
//! variable declarations, assignments, and expressions. It uses the `TypeChecker`
//! context to resolve types and unify them with expected types.

use common::Span;
use slynx_hir::{
    FieldMethod, HirExpression, HirExpressionKind, HirStatement, HirStatementKind, HirType, TypeId,
};

use crate::{Result, TypeChecker};

impl TypeChecker {
    /// Resolves a while statement, checking that the condition is a boolean and the body is a sequence of statements.
    fn resolve_while_statement(
        &mut self,
        condition: &mut HirExpression,
        body: &mut [HirStatement],
        ty: &TypeId,
    ) -> Result<()> {
        condition.ty = self.get_type_of_expr(condition)?;
        let bool_id = self.types_module.bool_id();

        condition.ty = self.unify(&condition.ty, &bool_id, &condition.span)?;

        self.resolve_statements(body, ty)?;
        Ok(())
    }

    /// Resolves an assignment statement, checking that the types match.
    fn resolve_assign_statement(
        &mut self,
        lhs: &mut HirExpression,
        value: &mut HirExpression,
        span: &Span,
    ) -> Result<()> {
        let refty = {
            let reader = self.types_module.get_type(&lhs.ty);
            match &*reader {
                HirType::Field(FieldMethod::Type(_, _))
                | HirType::Field(FieldMethod::Tuple(_, _)) => lhs.ty,
                HirType::Field(FieldMethod::Variable(..)) => {
                    let HirExpressionKind::FieldAccess {
                        ref mut field_index,
                        ..
                    } = lhs.kind
                    else {
                        unreachable!();
                    };
                    drop(reader);
                    let index = *field_index;
                    let (ty, index) = self.resolve_field_access_type(lhs.ty, index, &lhs.span)?;
                    *field_index = index;
                    lhs.ty = ty;
                    lhs.ty
                }
                HirType::VarReference(_) => lhs.ty,
                _ => unreachable!(),
            }
        };

        let ty = self.resolve(&lhs.ty, span)?;
        lhs.ty = refty;
        value.ty = self.get_type_of_expr(value)?;
        value.ty = self.unify(&ty, &value.ty, &value.span)?;
        Ok(())
    }

    pub(super) fn resolve_statement(
        &mut self,
        statement: &mut HirStatement,
        return_type: &TypeId,
    ) -> Result<()> {
        let span = &statement.span;
        match &mut statement.kind {
            HirStatementKind::While { condition, body } => {
                self.resolve_while_statement(condition, body, return_type)?
            }

            HirStatementKind::Variable { name, value } => {
                value.ty = self.get_type_of_expr(value)?;
                self.types_module.insert_variable(*name, value.ty);
            }

            HirStatementKind::Return { expr } => {
                expr.ty = self.get_type_of_expr(expr)?;
                expr.ty = self.unify(&expr.ty, return_type, span)?;
            }

            HirStatementKind::Expression { expr } => {
                expr.ty = self.get_type_of_expr(expr)?;
            }

            HirStatementKind::Assign { lhs, value } => {
                self.resolve_assign_statement(lhs, value, span)?
            }
        }
        Ok(())
    }

    /// Resolves the types of the provided `statements`.
    pub(super) fn resolve_statements(
        &mut self,
        statements: &mut [HirStatement],
        return_type: &TypeId,
    ) -> Result<()> {
        for statement in statements {
            self.resolve_statement(statement, return_type)?;
        }

        Ok(())
    }

    /// Helper to resolve a branch (then/else) that may be present or absent.
    /// Accepts `Option<&mut Vec<HirStatement>>` because the else branch is optional
    /// in the HIR representation. `expected` is the expected TypeId for statements.
    pub(crate) fn resolve_branch(
        &mut self,
        branch: Option<&mut Vec<HirStatement>>,
        expected: &TypeId,
    ) -> Result<TypeId> {
        let Some(stmts) = branch else {
            return Ok(self.types_module.void_id());
        };
        let Some((last, rest)) = stmts.split_last_mut() else {
            return Ok(self.types_module.void_id());
        };
        for stmt in rest {
            self.default_statement(stmt, expected)?;
        }
        match &mut last.kind {
            HirStatementKind::Expression { expr } => self.get_type_of_expr(expr),
            _ => Ok(self.types_module.void_id()),
        }
    }
}
