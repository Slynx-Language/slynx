//! The ownership analysis pass.
//!
//! Walks function bodies and validates move semantics and borrow rules.

use std::collections::HashMap;

use common::{Span, Spanned, pool::PoolId};

use crate::{
    DeclarationId, HirExpression, HirExpressionKind, HirFunctionDeclaration, HirStatement,
    SlynxHir, VariableId, model::HirPlace,
};

use super::{
    BorrowKind, ExpressionUse, FunctionOwnershipState, OwnershipError, OwnershipErrorKind,
};

/// The result of ownership analysis on the entire HIR.
#[derive(Debug, Default)]
pub struct OwnershipAnalysis {
    /// For each function, its ownership state at the end of analysis.
    pub function_states: HashMap<DeclarationId<HirFunctionDeclaration>, FunctionOwnershipState>,
    /// For each expression pool ID, what kind of use this is.
    pub expression_uses: HashMap<PoolId<HirExpression>, ExpressionUse>,
    /// Maps expressions to their corresponding place (if applicable).
    pub expression_places: HashMap<PoolId<HirExpression>, PoolId<HirPlace>>,
    /// Errors collected during analysis.
    pub errors: Vec<OwnershipError>,
}

impl OwnershipAnalysis {
    /// Look up how an expression is used (Move, Copy, Read, etc.).
    /// Returns `None` if the expression is not an identifier use.
    pub fn expression_use(&self, expr_id: PoolId<HirExpression>) -> Option<ExpressionUse> {
        self.expression_uses.get(&expr_id).copied()
    }
}

impl OwnershipAnalysis {
    /// Create a new, empty ownership analysis.
    pub fn new() -> Self {
        Self {
            function_states: HashMap::new(),
            expression_uses: HashMap::new(),
            expression_places: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Run ownership analysis on the entire HIR.
    pub fn analyze(hir: &SlynxHir) -> Self {
        let mut analysis = Self::new();
        for file in hir.files.iter() {
            for (id, func) in file.declarations.functions.iter().with_ids() {
                let func_id = DeclarationId::new(file.file, id);
                analysis.analyze_function(hir, func_id, func);
            }
        }
        analysis
    }

    /// Analyze a single function body.
    fn analyze_function(
        &mut self,
        hir: &SlynxHir,
        func_id: DeclarationId<HirFunctionDeclaration>,
        func: &HirFunctionDeclaration,
    ) {
        let mut state = FunctionOwnershipState::new();

        for arg in &func.args {
            state.get_variable_state(*arg);
        }
        for stmt in &func.statements {
            self.analyze_statement(hir, stmt, &mut state);
        }

        self.function_states.insert(func_id, state);
    }

    /// Analyzes a statement.
    fn analyze_statement(
        &mut self,
        hir: &SlynxHir,
        stmt: &Spanned<PoolId<HirStatement>>,
        state: &mut FunctionOwnershipState,
    ) {
        let stmt_data = &hir[stmt.data];
        match stmt_data {
            HirStatement::Variable { name, value } => {
                // RHS consumes (moves) variables
                self.analyze_expression(hir, value, state);
                state.get_variable_state(*name);
            }
            HirStatement::Assign { lhs, value } => {
                self.analyze_expression_as_place(hir, lhs, state);
                // RHS consumes (moves) variables
                self.analyze_expression(hir, value, state);
            }
            HirStatement::Expression { expr } => {
                self.analyze_expression(hir, expr, state);
            }
            HirStatement::Return { expr } => {
                if let Some(expr) = expr {
                    self.analyze_expression(hir, expr, state);
                }
            }
            HirStatement::While { condition, body } => {
                self.analyze_expression_read(hir, condition, state);
                for stmt in body {
                    self.analyze_statement(hir, stmt, state);
                }
            }
        }
    }

    /// Analyze an expression that is used as a place (lvalue).
    fn analyze_expression_as_place(
        &mut self,
        hir: &SlynxHir,
        expr: &Spanned<PoolId<HirExpression>>,
        state: &mut FunctionOwnershipState,
    ) {
        let expr_data = &hir[expr.data];
        match &expr_data.kind {
            HirExpressionKind::Identifier(id) => {
                if state.is_variable_moved(*id) {
                    self.errors.push(OwnershipError {
                        kind: OwnershipErrorKind::UseAfterMove { variable: *id },
                        span: expr.span,
                    });
                }
            }
            HirExpressionKind::FieldAccess { expr: parent, .. } => {
                self.analyze_expression_as_place(hir, parent, state);
            }
            HirExpressionKind::Deref(inner) => {
                self.analyze_expression_as_place(hir, inner, state);
            }
            _ => {}
        }
    }

    /// Analyze a consuming expression (assignment RHS, function arg, return value), if the value is being used as a place, then its moved. This is not true for expressions whose types are &/&mut
    fn analyze_expression(
        &mut self,
        hir: &SlynxHir,
        expr: &Spanned<PoolId<HirExpression>>,
        state: &mut FunctionOwnershipState,
    ) {
        let expr_data = &hir[expr.data];
        match &expr_data.kind {
            HirExpressionKind::Identifier(id) => {
                let use_kind = self.analyze_variable_use(*id, expr.span, state);
                self.expression_uses.insert(expr.data, use_kind);
            }
            //HirExpressionKind::Static { id } => {}
            HirExpressionKind::Reference(inner) => {
                let mutable = hir.view(expr.data).ty_viewer().is_mutable_ref().is_some();
                self.analyze_reference(hir, inner, mutable, expr.span, state);
            }
            HirExpressionKind::FunctionCall { args, name, .. } => {
                let ty = hir.get_file(name.file_id)[name.local_id].ty;
                let viewer = hir.view(ty);
                let ty_viewer = viewer
                    .is_function()
                    .expect("View of the type of a function should be a function type");

                for (arg, param) in args.iter().zip(ty_viewer.arguments()) {
                    if hir.view(*param).is_ref() {
                        self.analyze_expression_read(hir, arg, state);
                    } else {
                        self.analyze_expression(hir, arg, state);
                    }
                }
            }
            HirExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.analyze_expression_read(hir, condition, state);
                for stmt in then_branch {
                    self.analyze_statement(hir, stmt, state);
                }
                if let Some(else_branch) = else_branch {
                    for stmt in else_branch {
                        self.analyze_statement(hir, stmt, state);
                    }
                }
            }
            // All other expression kinds are composite — their sub-expressions
            // are READ, not consumed. Delegate to analyze_expression_read.
            _ => self.analyze_expression_read(hir, expr, state),
        }
    }

    /// Analyze a non-consuming expression (field access parent, binary operand,
    /// array index container, tuple/object/array elements, while condition, etc.).
    /// Identifiers referenced here are READ (not moved).
    fn analyze_expression_read(
        &mut self,
        hir: &SlynxHir,
        expr: &Spanned<PoolId<HirExpression>>,
        state: &mut FunctionOwnershipState,
    ) {
        let expr_data = &hir[expr.data];
        match &expr_data.kind {
            HirExpressionKind::Identifier(id) => {
                // Read-only: check for use-after-move, but do NOT mark as moved
                if state.is_variable_moved(*id) {
                    self.errors.push(OwnershipError {
                        kind: OwnershipErrorKind::UseAfterMove { variable: *id },
                        span: expr.span,
                    });
                }
                self.expression_uses.insert(expr.data, ExpressionUse::Read);
            }
            HirExpressionKind::FieldAccess { expr: parent, .. } => {
                self.analyze_expression_read(hir, parent, state);
            }
            HirExpressionKind::Deref(inner) => {
                self.analyze_expression_read(hir, inner, state);
            }
            HirExpressionKind::Reference(inner) => {
                // Taking a reference reads the inner, doesn't consume it
                self.analyze_expression_read(hir, inner, state);
            }
            HirExpressionKind::Binary { lhs, rhs, .. } => {
                self.analyze_expression_read(hir, lhs, state);
                self.analyze_expression_read(hir, rhs, state);
            }
            HirExpressionKind::ArrayIndex(arr, index) => {
                self.analyze_expression_read(hir, arr, state);
                self.analyze_expression_read(hir, index, state);
            }
            HirExpressionKind::Tuple(elements) => {
                for elem in elements {
                    self.analyze_expression_read(hir, elem, state);
                }
            }
            HirExpressionKind::Object { fields, .. } => {
                for field in fields {
                    self.analyze_expression_read(hir, field, state);
                }
            }
            HirExpressionKind::Array(elements) | HirExpressionKind::Vector(elements) => {
                for elem in elements {
                    self.analyze_expression_read(hir, elem, state);
                }
            }
            HirExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.analyze_expression_read(hir, condition, state);
                for stmt in then_branch {
                    self.analyze_statement(hir, stmt, state);
                }
                if let Some(else_branch) = else_branch {
                    for stmt in else_branch {
                        self.analyze_statement(hir, stmt, state);
                    }
                }
            }
            // Literals and other pure expressions don't affect ownership
            _ => {}
        }
    }

    /// Analyze a variable being used as a move.
    fn analyze_variable_use(
        &mut self,
        id: VariableId,
        span: Span,
        state: &mut FunctionOwnershipState,
    ) -> ExpressionUse {
        if state.is_variable_moved(id) {
            self.errors.push(OwnershipError {
                kind: OwnershipErrorKind::UseAfterMove { variable: id },
                span,
            });
        } else {
            state.mark_variable_moved(id);
        }
        ExpressionUse::Move
    }

    /// Analyze a reference being taken.
    fn analyze_reference(
        &mut self,
        hir: &SlynxHir,
        inner: &Spanned<PoolId<HirExpression>>,
        mutable: bool,
        span: Span,
        state: &mut FunctionOwnershipState,
    ) {
        let kind = if mutable {
            BorrowKind::Mutable
        } else {
            BorrowKind::Immutable
        };

        if let Some(place_id) = self.build_place_from_expr(hir, inner.data)
            && let HirPlace::Variable(id) = &hir.places[place_id]
        {
            match () {
                _ if state.can_borrow_variable(*id, kind) => state.borrow_variable(*id, kind),
                _ if state.is_variable_moved(*id) => {
                    self.errors.push(OwnershipError {
                        kind: OwnershipErrorKind::UseAfterMove { variable: *id },
                        span,
                    });
                    return;
                }
                _ => {
                    let existing_kind = if state
                        .variable_states
                        .get(id)
                        .map(|s| s.borrowed_mut > 0)
                        .unwrap_or(false)
                    {
                        BorrowKind::Mutable
                    } else {
                        BorrowKind::Immutable
                    };
                    self.errors.push(OwnershipError {
                        kind: OwnershipErrorKind::ConflictingBorrow {
                            variable: *id,
                            existing_borrow: existing_kind,
                            new_borrow: kind,
                        },
                        span,
                    });
                    return;
                }
            }
        }

        // The inner expression is READ, not consumed (taking a ref borrows it)
        self.analyze_expression_read(hir, inner, state);
    }
}
