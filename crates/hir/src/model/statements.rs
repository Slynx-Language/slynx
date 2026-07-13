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

use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};

use crate::{HirType, SymbolPointer, VariableId, model::HirExpression};

#[derive(Debug)]
///A Style definition when declaring a stylesheet
pub struct StylesDefinition {
    ///The name of the style
    pub name: SymbolPointer,
    ///The expression related to it
    pub expr: Spanned<PoolId<HirExpression>>,
    ///The type it should have. Used on Type checker
    pub expected_type: DedupPoolId<HirType>,
    ///The span of this definition
    pub span: Span,
}

impl StylesDefinition {
    ///Creates a new style definition with the given `symb` name, `expr` and `expected_type`. The given `expected_type` may not be the same as `expr` type when created,
    ///but that'll be asserted on Type checking
    pub fn new(
        symb: SymbolPointer,
        expr: Spanned<PoolId<HirExpression>>,
        expected_type: DedupPoolId<HirType>,
        span: Span,
    ) -> Self {
        Self {
            name: symb,
            expr,
            expected_type,
            span,
        }
    }
}

#[derive(Debug)]
///The kind of block to apply the stylesheet. This refers to the event that is happening on the component that uses the style.
pub enum HirStyleBlockKind {
    ///The default style to be applied
    Default,
    ///The style to be applied when the component is hovered
    Hover,
}

#[derive(Debug)]
///A style block containing information about the styles to apply and when the styles should be applied
pub struct HirStyleBlock {
    ///The kind of the block, thus, when the `definitions` should be applied
    pub kind: HirStyleBlockKind,
    ///The styles definitions to apply to the component
    pub definitions: Vec<StylesDefinition>,
}

#[derive(Debug)]
///An statement that might occurr inside a stylesheet
pub enum HirStyleStatement {
    ///A normal statement
    Statement(Spanned<PoolId<HirStatement>>),
    ///Styling definitions
    Styles(Vec<HirStyleBlock>),
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
#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub enum HirStatement {
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
        lhs: Spanned<PoolId<HirExpression>>,

        /// The value to assign.
        ///
        /// Evaluated before assignment, then stored in the target location.
        value: Spanned<PoolId<HirExpression>>,
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
        value: Spanned<PoolId<HirExpression>>,
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
        expr: Spanned<PoolId<HirExpression>>,
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
    /// - `expr` — The expression to return, or `None` for void returns
    Return {
        /// The value to return from the function, if any.
        ///
        /// Must match the function's declared return type (or be absent for
        /// functions that don't return a value).
        expr: Option<Spanned<PoolId<HirExpression>>>,
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
        condition: Spanned<PoolId<HirExpression>>,

        /// The loop body statements.
        ///
        /// Executed in order each time the condition is true.
        /// The body executes in its own scope for variable bindings.
        body: Vec<Spanned<PoolId<HirStatement>>>,
    },
}
