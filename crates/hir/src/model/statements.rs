//! Statement Nodes
//!
//! This module defines the [`HirStatement`] structure and its variants,
//! representing statements and declarations in the HIR.
//!
//! # Overview
//!
//! Statements represent actions and declarations in Slynx programs. Unlike
//! expressions, which always produce a value, statements are executed for
//! their side effects or to introduce new bindings.
//!
//! # Statement Types
//!
//! ## Variable Declarations
//! - [`Variable`](HirStatementKind::Variable) — `let` and `let mut` bindings
//!
//! ## Assignments
//! - [`Assign`](HirStatementKind::Assign) — Assignment to existing variables
//!
//! ## Control Flow
//! - [`While`](HirStatementKind::While) — While loops
//! - [`Return`](HirStatementKind::Return) — Function returns (implicit or explicit)
//!
//! ## Expressions
//! - [`Expression`](HirStatementKind::Expression) — Expression used as a statement
//!
//! # Key Differences from Expressions
//!
//! 1. **No Required Value** — Statements don't necessarily produce a value
//! 2. **Side Effects** — Statements are executed for their effects (binding, mutation, control flow)
//! 3. **Implicit Returns** — The last expression in a function body becomes a return statement
//!
//! # Examples
//!
//! ```rust
//! # use slynx_frontend::hir::model::*;
//! # use common::Span;
//! # use crate::slynx_frontend::hir::{VariableId, ExpressionId, TypeId};
//! # let span = Span::default();
//! # let var_id = VariableId::new();
//! # let expr = HirExpression {
//! #     id: ExpressionId::new(),
//! #     ty: TypeId::from_raw(0),
//! #     kind: HirExpressionKind::Int(42),
//! #     span,
//! # };
//!
//! // Variable declaration
//! let var_stmt = HirStatement::new_variable(var_id, expr, span);
//!
//! // Assignment
//! let assign_stmt = HirStatement {
//!     kind: HirStatementKind::Assign {
//!         lhs: lhs_expr,
//!         value: rhs_expr,
//!     },
//!     span,
//! };
//!
//! // While loop
//! let while_stmt = HirStatement::new_while(condition_expr, body_stmts, span);
//!
//! // Return (implicit from last expression in function body)
//! let return_stmt = HirStatement::new_return(expr);
//! ```
//!
//! # Implicit Returns
//!
//! In Slynx, the last expression in a function body is implicitly returned:
//!
//! ```slynx
//! func add(a: int, b: int): int {
//!     a + b  // Implicitly returned
//! }
//! ```
//!
//! During HIR generation, this expression is wrapped in a [`Return`] statement.

use common::Span;

use crate::{VariableId, model::HirExpression};

/// A statement node in the HIR.
///
/// Statements represent actions, declarations, and control flow constructs.
/// Unlike expressions, statements are executed for side effects rather than
/// producing values (though they may contain expressions that do).
///
/// # Fields
///
/// - `kind` — What kind of statement this is (declaration, assignment, loop, etc.)
/// - `span` — The source location of this statement
///
/// # Immutability
///
/// Statements are immutable once created. They form part of the HIR declaration
/// tree and are not modified after generation.
///
/// # Sequences
///
/// Statements are typically executed in sequence, as they appear in the source.
/// The order matters for:
/// - Variable declarations (must precede use)
/// - Side effects (order of evaluation)
/// - Control flow (loop bodies, conditional branches)
#[derive(Debug)]
#[repr(C)]
pub struct HirStatement {
    /// What kind of statement this is.
    ///
    /// The kind determines the statement's behavior and what additional data
    /// it contains (e.g., variable bindings for declarations, conditions for loops).
    pub kind: HirStatementKind,

    /// The source location of this statement.
    ///
    /// Used for error reporting and IDE features. Preserved throughout the
    /// compilation pipeline for accurate diagnostics.
    pub span: Span,
}

/// The kind of a statement.
///
/// This enum describes all possible statement forms in Slynx. Each variant
/// represents a different kind of action or declaration.
///
/// # Categories
///
/// ## Variable Declarations
/// Introduce new bindings in the current scope.
///
/// - [`Variable`](HirStatementKind::Variable) — `let` and `let mut` bindings
///
/// ## Assignments
/// Modify existing bindings.
///
/// - [`Assign`](HirStatementKind::Assign) — Assignment to mutable variables
///
/// ## Control Flow
/// Affect the order of execution.
///
/// - [`While`](HirStatementKind::While) — While loops
/// - [`Return`](HirStatementKind::Return) — Function returns
///
/// ## Expressions
/// Standalone expressions (often for side effects).
///
/// - [`Expression`](HirStatementKind::Expression) — Expression used as a statement
///
/// # Note on Returns
///
/// Functions in Slynx implicitly return the value of their last expression.
/// During HIR generation, this final expression is wrapped in a `Return`
/// statement. Explicit `return` statements are also supported and lowered
/// to the same `Return` variant.
#[derive(Debug)]
#[repr(C)]
pub enum HirStatementKind {
    /// Assignment statement.
    ///
    /// Assigns a value to a mutable variable.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let mut x = 0;
    /// x = 42;  // Assignment
    /// ```
    ///
    /// # Fields
    ///
    /// - `lhs` — The left-hand side expression (must be an identifier or field access)
    /// - `value` — The right-hand side expression to assign
    Assign {
        /// The target of the assignment (what to assign to).
        ///
        /// Typically an [`Identifier`](HirExpressionKind::Identifier) expression,
        /// but can also be a field access for struct field assignment.
        lhs: HirExpression,

        /// The value to assign.
        ///
        /// Evaluated before assignment, then stored in the target location.
        value: HirExpression,
    },

    /// Variable declaration statement.
    ///
    /// Introduces a new binding in the current scope. Can be either mutable
    /// (`let mut`) or immutable (`let`).
    ///
    /// # Example
    ///
    /// ```slynx
    /// let x = 42;           // Immutable
    /// let mut y = x + 1;    // Mutable
    /// ```
    ///
    /// # Fields
    ///
    /// - `name` — The variable's unique ID
    /// - `value` — The initializer expression
    Variable {
        /// The unique identifier for this variable.
        ///
        /// Used to reference the variable in later expressions within the same scope.
        name: VariableId,

        /// The initializer expression.
        ///
        /// Evaluated at the point of declaration to produce the variable's initial value.
        /// The type of this expression determines the variable's type (unless explicitly
        /// annotated, which is handled during type checking).
        value: HirExpression,
    },

    /// Expression used as a statement.
    ///
    /// Evaluates an expression for its side effects, discarding the result.
    /// Common for function calls that return `void` or have important side effects.
    ///
    /// # Example
    ///
    /// ```slynx
    /// print("Hello");  // Function call for side effect
    /// do_something(); // Another statement-expression
    /// ```
    ///
    /// # Fields
    ///
    /// - `expr` — The expression to evaluate
    Expression {
        /// The expression to evaluate as a statement.
        ///
        /// The result is discarded unless this is the last expression in a function body,
        /// in which case it becomes an implicit return.
        expr: HirExpression,
    },

    /// Return statement.
    ///
    /// Exits the current function and optionally returns a value.
    ///
    /// # Note
    ///
    /// In Slynx, the last expression in a function body is implicitly returned.
    /// During HIR generation, this expression is wrapped in a `Return` statement.
    /// Explicit `return` statements are also lowered to this variant.
    ///
    /// # Example
    ///
    /// ```slynx
    /// func explicit_return(): int {
    ///     return 42;  // Explicit return
    /// }
    ///
    /// func implicit_return(): int {
    ///     42  // Implicit return (wrapped in Return during HIR gen)
    /// }
    ///
    /// func no_return(): void {
    ///     print("done");
    ///     // Returns void implicitly
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `expr` — The expression to return (may be `void`)
    Return {
        /// The value to return from the function.
        ///
        /// Must match the function's declared return type (or be `void` for
        /// functions that don't return a value).
        expr: HirExpression,
    },

    /// While loop statement.
    ///
    /// Repeatedly executes a block while a condition is true.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let mut counter = 10;
    /// while counter > 0 {
    ///     print(counter);
    ///     counter = counter - 1;
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `condition` — The loop condition (must be boolean)
    /// - `body` — Statements to execute each iteration
    While {
        /// The loop condition expression.
        ///
        /// Evaluated before each iteration. If false, the loop exits.
        /// Must evaluate to a boolean value.
        condition: HirExpression,

        /// The loop body statements.
        ///
        /// Executed in order each time the condition is true.
        /// The body executes in its own scope for variable bindings.
        body: Vec<HirStatement>,
    },
}

impl HirStatement {
    /// Creates a new return statement.
    ///
    /// This is used for both explicit `return` statements and implicit returns
    /// from the last expression in a function body.
    ///
    /// # Arguments
    ///
    /// * `expr` — The expression to return
    ///
    /// # Returns
    ///
    /// A new [`HirStatement`] with [`HirStatementKind::Return`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::*;
    /// # use common::Span;
    /// # let span = Span::default();
    /// # let expr = HirExpression {
    /// #     id: ExpressionId::new(),
    /// #     ty: TypeId::from_raw(0),
    /// #     kind: HirExpressionKind::Int(42),
    /// #     span,
    /// # };
    /// let return_stmt = HirStatement::new_return(expr);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`HirStatementKind::Return`]
    /// - [`crate::hir::implementation::declarations::resolve_function`]
    pub fn new_return(expr: HirExpression) -> Self {
        Self {
            span: expr.span,
            kind: HirStatementKind::Return { expr },
        }
    }

    /// Creates a new variable declaration statement.
    ///
    /// Used for both `let` and `let mut` declarations during HIR generation.
    /// The variable's mutability is tracked separately through scope management.
    ///
    /// # Arguments
    ///
    /// * `name` — The variable's unique ID
    /// * `value` — The initializer expression
    /// * `span` — The source location of the declaration
    ///
    /// # Returns
    ///
    /// A new [`HirStatement`] with [`HirStatementKind::Variable`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::*;
    /// # use common::Span;
    /// # use crate::slynx_frontend::hir::{VariableId, ExpressionId, TypeId};
    /// # let span = Span::default();
    /// # let var_id = VariableId::new();
    /// # let value = HirExpression {
    /// #     id: ExpressionId::new(),
    /// #     ty: TypeId::from_raw(0),
    /// #     kind: HirExpressionKind::Int(42),
    /// #     span,
    /// # };
    /// let var_stmt = HirStatement::new_variable(var_id, value, span);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`HirStatementKind::Variable`]
    /// - [`crate::hir::modules::ScopeModule::create_variable`]
    pub fn new_variable(name: VariableId, value: HirExpression, span: Span) -> Self {
        Self {
            kind: HirStatementKind::Variable { name, value },
            span,
        }
    }

    /// Creates a new while loop statement.
    ///
    /// # Arguments
    ///
    /// * `condition` — The loop condition expression
    /// * `body` — The statements to execute each iteration
    /// * `span` — The source location of the loop
    ///
    /// # Returns
    ///
    /// A new [`HirStatement`] with [`HirStatementKind::While`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::*;
    /// # use common::Span;
    /// # use crate::slynx_frontend::hir::{ExpressionId, TypeId};
    /// # let span = Span::default();
    /// # let condition = HirExpression {
    /// #     id: ExpressionId::new(),
    /// #     ty: TypeId::from_raw(0),
    /// #     kind: HirExpressionKind::Bool(true),
    /// #     span,
    /// # };
    /// # let body = vec![];
    /// let while_stmt = HirStatement::new_while(condition, body, span);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`HirStatementKind::While`]
    /// - [`crate::hir::implementation::statements::resolve_statement`]
    pub fn new_while(condition: HirExpression, body: Vec<HirStatement>, span: Span) -> Self {
        Self {
            span,
            kind: HirStatementKind::While { condition, body },
        }
    }

    /// Creates a new expression statement.
    ///
    /// This is used when an expression is used as a statement, typically for
    /// its side effects (e.g., function calls).
    ///
    /// # Arguments
    ///
    /// * `expr` — The expression to evaluate
    ///
    /// # Returns
    ///
    /// A new [`HirStatement`] with [`HirStatementKind::Expression`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::*;
    /// # use common::Span;
    /// # use crate::slynx_frontend::hir::{ExpressionId, TypeId};
    /// # let span = Span::default();
    /// # let expr = HirExpression {
    /// #     id: ExpressionId::new(),
    /// #     ty: TypeId::from_raw(0),
    /// #     kind: HirExpressionKind::Int(42),
    /// #     span,
    /// # };
    /// let expr_stmt = HirStatement::new_expression(expr);
    /// ```
    ///
    /// # Note
    ///
    /// If this is the last expression in a function body, it will be wrapped
    /// in a `Return` statement instead of an `Expression` statement.
    ///
    /// # See Also
    ///
    /// - [`HirStatementKind::Expression`]
    /// - [`crate::hir::implementation::statements::resolve_statement`]
    pub fn new_expression(expr: HirExpression) -> Self {
        Self {
            span: expr.span,
            kind: HirStatementKind::Expression { expr },
        }
    }
}
