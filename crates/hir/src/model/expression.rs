//! Expression Nodes
//!
//! This module defines the [`HirExpression`] structure and its variants,
//! representing all possible expressions in the HIR.
//!
//! # Overview
//!
//! Expressions are the fundamental building blocks of computation in Slynx.
//! They produce values and can be composed to form more complex computations.
//! In the HIR, every expression has:
//!
//! - A unique [`ExpressionId`] for identification
//! - A [`DedupPoolId<HirType>`] representing its type
//! - A [`HirExpressionKind`] describing what kind of expression it is
//! - A source [`Span`] for error reporting
//!
//! # Categories
//!
//! ## Literals
//! - [`Int`](HirExpressionKind::Int) — Integer literals
//! - [`Float`](HirExpressionKind::Float) — Floating-point literals
//! - [`StringLiteral`](HirExpressionKind::StringLiteral) — String literals
//! - [`Bool`](HirExpressionKind::Bool) — Boolean literals
//!
//! ## Composite
//! - [`Tuple`](HirExpressionKind::Tuple) — Tuple expressions
//! - [`Object`](HirExpressionKind::Object) — Object construction
//! - [`Component`](HirExpressionKind::Component) — Component construction
//!
//! ## Operations
//! - [`Binary`](HirExpressionKind::Binary) — Binary operations
//! - [`FieldAccess`](HirExpressionKind::FieldAccess) — Field access
//! - [`FunctionCall`](HirExpressionKind::FunctionCall) — Function calls
//!
//! ## Control Flow
//! - [`If`](HirExpressionKind::If) — Conditional expressions
//!
//! ## References
//! - [`Identifier`](HirExpressionKind::Identifier) — Variable references
//! - [`Specialized`](HirExpressionKind::Specialized) — Specialized components
//!
//! # Examples
//!
//! ```rust
//! # use slynx_frontend::hir::model::*;
//! # use common::Span;
//! # let span = Span::default();
//!
//! // Integer literal
//! let expr = HirExpression {
//!     id: ExpressionId::new(),
//!     ty: DedupPoolId<HirType>::from_raw(0), // int type
//!     kind: HirExpressionKind::Int(42),
//!     span,
//! };
//!
//! // Binary operation: a + b
//! let add_expr = HirExpression {
//!     id: ExpressionId::new(),
//!     ty: DedupPoolId<HirType>::from_raw(0),
//!     kind: HirExpressionKind::Binary {
//!         lhs: Box::new(a_expr),
//!         op: Operator::Add,
//!         rhs: Box::new(b_expr),
//!     },
//!     span,
//! };
//!
//! // Function call
//! let call_expr = HirExpression {
//!     id: ExpressionId::new(),
//!     ty: DedupPoolId<HirType>::from_raw(0),
//!     kind: HirExpressionKind::FunctionCall {
//!         name: DeclarationId::new(),
//!         args: vec![arg1, arg2],
//!     },
//!     span,
//! };
//! ```
//!
//! # See Also
//!
//! - [`HirExpressionKind`] — Enum of all expression kinds
//! - [`crate::hir::model::HirStatement`] — Statement nodes
//! - [`crate::hir::implementation::expression::resolve_expr`] — Expression resolution

use crate::{
    DeclarationId, HirFunctionDeclaration, HirStaticDeclaration, HirType, SymbolPointer,
    VariableId, model::HirStatement,
};
use common::{
    Operator, Spanned,
    pool::{DedupPoolId, PoolId},
};
use ordered_float::OrderedFloat;

/// A property assignment within a component construction expression.
///
/// Links a property index (position within the component's property list) to
/// the [`HirExpression`] that computes its value at runtime.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PropertyExpression {
    /// The property index this expression will be applied to
    index: usize,
    /// The expression of the property
    expr: Spanned<PoolId<HirExpression>>,
}

impl PropertyExpression {
    /// Creates a new [`PropertyExpression`] with the given property `index` and its value `expr`.
    pub fn new(index: usize, expr: Spanned<PoolId<HirExpression>>) -> Self {
        Self { index, expr }
    }

    /// Returns the property index this expression is assigned to.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns a shared reference to the property value expression.
    pub fn expr(&self) -> &Spanned<PoolId<HirExpression>> {
        &self.expr
    }
}

#[derive(Debug)]
pub struct HirComponentExpression {
    /// The type of the component.
    pub name: DedupPoolId<HirType>,
    /// The properties of this component
    pub properties: Vec<PropertyExpression>,
    /// The children of this component
    pub children: Vec<Spanned<PoolId<HirComponentExpression>>>,
}

/// An expression node in the HIR.
///
/// Every expression in the HIR is represented by this structure, which
/// contains the expression's kind, type, unique identifier, and source location.
///
/// # Fields
///
/// - `id` — A unique identifier for this expression
/// - `ty` — The expression's type, as a [`DedupPoolId<HirType>`]
/// - `kind` — What kind of expression this is (literal, binary, call, etc.)
/// - `span` — The source location of this expression
///
/// # Immutability
///
/// Expressions are typically immutable once created. The HIR generation phase
/// constructs expressions and stores them in the declaration tree.
///
/// # Type Information
///
/// Every expression has a type, even if it's the special `Infer` type that
/// will be filled in during type checking. This allows for early detection
/// of type errors and better code completion.
///
/// # Examples
///
/// ```rust
/// # use slynx_frontend::hir::model::*;
/// # use common::{Operator, Span};
/// # use crate::slynx_frontend::hir::DedupPoolId<HirType>;
/// # let span = Span::default();
/// # let int_type = DedupPoolId<HirType>::from_raw(0);
/// # let lhs = HirExpression {
/// #     id: ExpressionId::new(),
/// #     ty: int_type,
/// #     kind: HirExpressionKind::Int(1),
/// #     span,
/// # };
/// # let rhs = HirExpression {
/// #     id: ExpressionId::new(),
/// #     ty: int_type,
/// #     kind: HirExpressionKind::Int(2),
/// #     span,
/// # };
///
/// // Create a binary addition expression
/// let add_expr = HirExpression {
///     id: ExpressionId::new(),
///     ty: int_type,
///     kind: HirExpressionKind::Binary {
///         lhs: Box::new(lhs),
///         op: Operator::Add,
///         rhs: Box::new(rhs),
///     },
///     span,
/// };
/// ```
///
/// # Implementation Notes
///
/// - The `#[repr(C)]` attribute ensures a predictable memory layout, which is
///   important for FFI and stable ABI
/// - Expressions form a tree structure through the use of `Box` for sub-expressions
/// - The `ExpressionId` is used for tracking and debugging, but doesn't affect
///   the semantics of the program
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HirExpression {
    /// The type of this expression.
    ///
    /// Every expression has a type, which may be:
    /// - A concrete type (int, bool, etc.)
    /// - A reference to a user-defined type
    /// - The special `Infer` type, indicating the type should be inferred
    ///
    /// This field is used during type checking to ensure type correctness.
    pub ty: DedupPoolId<HirType>,

    /// The kind of expression this is.
    ///
    /// This enum determines what the expression represents and what sub-expressions
    /// or values it contains. See [`HirExpressionKind`] for all possible variants.
    pub kind: HirExpressionKind,
}

/// The kind of an expression.
///
/// This enum describes all possible expression forms in the Slynx language.
/// Each variant represents a different way to produce a value, from simple
/// literals to complex function calls and object constructions.
///
/// # Categories
///
/// ## Literals
/// Simple values written directly in source code.
///
/// - [`Int`](HirExpressionKind::Int) — Integer literals like `42` or `-10`
/// - [`Float`](HirExpressionKind::Float) — Floating-point literals like `3.14` or `2.0`
/// - [`StringLiteral`](HirExpressionKind::StringLiteral) — String literals like `"hello"`
/// - [`Bool`](HirExpressionKind::Bool) — Boolean literals `true` and `false`
///
/// ## Composite
/// Expressions that combine multiple values into a single value.
///
/// - [`Tuple`](HirExpressionKind::Tuple) — Tuple expressions like `(1, "hello", true)`
/// - [`Object`](HirExpressionKind::Object) — Object construction like `Person(name: "Alice", age: 30)`
/// - [`Component`](HirExpressionKind::Component) — Component construction like `Button(label: "Click me")`
///
/// ## Operations
/// Expressions that perform computations.
///
/// - [`Binary`](HirExpressionKind::Binary) — Binary operations like `a + b` or `x && y`
/// - [`FieldAccess`](HirExpressionKind::FieldAccess) — Field access like `person.name`
/// - [`FunctionCall`](HirExpressionKind::FunctionCall) — Function calls like `print("hello")`
///
/// ## Control Flow
/// Expressions that control program execution.
///
/// - [`If`](HirExpressionKind::If) — Conditional expressions like `if condition { true } else { false }`
///
/// ## References
/// Expressions that refer to other values.
///
/// - [`Identifier`](HirExpressionKind::Identifier) — Variable references like `x`
/// - [`Specialized`](HirExpressionKind::Specialized) — Specialized component constructions
///
/// # Examples
///
/// ```rust
/// # use slynx_frontend::hir::model::*;
/// # use common::{Operator, Span};
/// # let span = Span::default();
/// # use crate::slynx_frontend::hir::{ExpressionId, DedupPoolId<HirType>};
/// # let int_type = DedupPoolId<HirType>::from_raw(0);
/// # let expr_id = ExpressionId::new();
///
/// // Integer literal expression
/// let int_expr = HirExpressionKind::Int(42);
///
/// // String literal expression
/// let str_expr = HirExpressionKind::StringLiteral("hello".to_string());
///
/// // Boolean literal expression
/// let bool_expr = HirExpressionKind::Bool(true);
///
/// // Tuple expression
/// let tuple_expr = HirExpressionKind::Tuple(vec![int_expr, bool_expr]);
///
/// // Binary operation expression
/// let binary_expr = HirExpressionKind::Binary {
///     lhs: Box::new(HirExpression { id: expr_id, ty: int_type, kind: int_expr, span }),
///     op: Operator::Add,
///     rhs: Box::new(HirExpression { id: expr_id, ty: int_type, kind: int_expr, span }),
/// };
///
/// // Function call expression
/// let call_expr = HirExpressionKind::FunctionCall {
///     name: DeclarationId::from_raw(0),
///     args: vec![HirExpression { id: expr_id, ty: int_type, kind: int_expr, span }],
/// };
///
/// // Field access expression
/// let field_expr = HirExpressionKind::FieldAccess {
///     expr: Box::new(HirExpression { id: expr_id, ty: int_type, kind: int_expr, span }),
///     field_index: 0,
/// };
/// ```
///
/// # Pattern Matching
///
/// You can use pattern matching to handle different expression kinds:
///
/// ```rust
/// # use slynx_frontend::hir::model::HirExpressionKind;
/// # let expr_kind = HirExpressionKind::Int(42);
/// match expr_kind {
///     HirExpressionKind::Int(value) => {
///         println!("Integer: {}", value);
///     }
///     HirExpressionKind::StringLiteral(s) => {
///         println!("String: {}", s);
///     }
///     HirExpressionKind::Binary { lhs, op, rhs } => {
///         println!("Binary operation: {:?}", op);
///     }
///     _ => {
///         println!("Other expression kind");
///     }
/// }
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum HirExpressionKind {
    /// An integer literal expression.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let x = 42;
    /// let y = -10;
    /// ```
    ///
    /// The integer type is 32-bit signed (`int`).
    Int(i32),

    /// A string literal expression.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let message = "Hello, world!";
    /// ```
    StringLiteral(SymbolPointer),

    /// A floating-point literal expression.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let pi = 3.14159;
    /// let gravity = 9.8;
    /// ```
    ///
    /// The float type is 32-bit (`float`).
    Float(OrderedFloat<f32>),

    True,
    False,

    /// A tuple expression.
    ///
    /// Tuples group multiple values of potentially different types into a
    /// single compound value. Elements are accessed by numeric index.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let point = (10, 20);
    /// let x = point.0;  // 10
    /// let y = point.1;  // 20
    /// ```
    ///
    /// # Fields
    ///
    /// - A vector of sub-expressions that make up the tuple elements
    Tuple(Vec<Spanned<PoolId<HirExpression>>>),

    /// A binary operation expression.
    ///
    /// Binary operations combine two expressions using an operator.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let sum = a + b;
    /// let is_equal = x == y;
    /// let condition = p && q;
    /// ```
    ///
    /// # Fields
    ///
    /// - `lhs` — The left-hand side expression
    /// - `op` — The binary operator
    /// - `rhs` — The right-hand side expression
    Binary {
        /// Left-hand side operand.
        lhs: Spanned<PoolId<HirExpression>>,

        /// The binary operator.
        op: Operator,

        /// Right-hand side operand.
        rhs: Spanned<PoolId<HirExpression>>,
    },

    /// An identifier expression (variable reference).
    ///
    /// References a variable by its unique ID.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let x = 42;
    /// let y = x;  // References variable x
    /// ```
    ///
    /// # Fields
    ///
    /// - The variable ID being referenced
    Identifier(VariableId),

    /// A component construction expression.
    ///
    /// Creates an instance of a user-defined component with specified
    /// property values and child members.
    ///
    /// # Example
    ///
    /// ```slynx
    /// component Button{
    ///     prop label: str = "Click me";
    /// }
    ///
    /// let button = Button {label: "Submit"};
    /// ```
    ///
    /// # Fields
    ///
    /// - `name` — The component's type ID
    /// - `values` — Property values and child members
    Component(Spanned<PoolId<HirComponentExpression>>),

    /// An object construction expression.
    ///
    /// Creates an instance of a user-defined object (struct) with specified
    /// field values.
    ///
    /// # Example
    ///
    /// ```slynx
    /// object Point { x: int, y: int }
    /// let p = Point(x: 10, y: 20);
    /// ```
    ///
    /// # Fields
    ///
    /// - `name` — The object's type ID
    /// - `fields` — Field value expressions in declaration order
    Object {
        /// The object's type ID.
        name: DedupPoolId<HirType>,

        /// Field value expressions, in declaration order.
        fields: Vec<Spanned<PoolId<HirExpression>>>,
    },

    /// A field access expression.
    ///
    /// Accesses a field of a struct, object, or variable.
    ///
    /// # Example
    ///
    /// ```slynx
    /// object Person { name: str, age: int }
    /// let p = Person(name: "Alice", age: 30);
    /// let name = p.name;   // Accesses field 0
    /// let age = p.age;     // Accesses field 1
    /// ```
    ///
    /// # Fields
    ///
    /// - `expr` — The expression whose field is being accessed
    /// - `field_index` — The index of the field within the containing type
    /// - `field_name` — The field's name (always set; used by codegen for externals)
    FieldAccess {
        /// The expression being accessed (e.g., `p` in `p.name`).
        expr: Spanned<PoolId<HirExpression>>,

        /// The index of the field within the struct or object type.
        field_index: usize,

        /// The field's name symbol. Always present for named fields; used by
        /// codegen to emit `dyn_prop_get`/`dyn_prop_set` for external types.
        field_name: Option<SymbolPointer>,
    },

    /// A function call expression.
    ///
    /// Invokes a function with the provided arguments.
    ///
    /// # Example
    ///
    /// ```slynx
    /// func add(a: int, b: int): int {
    ///     a + b
    /// }
    ///
    /// let result = add(3, 5);  // Calls function add
    /// ```
    ///
    /// # Fields
    ///
    /// - `name` — The function's declaration ID
    /// - `args` — Argument expressions
    FunctionCall {
        /// The declaration ID of the function being called.
        name: DeclarationId<HirFunctionDeclaration>,

        /// The argument expressions passed to the function.
        args: Vec<Spanned<PoolId<HirExpression>>>,
    },
    MethodCall {
        parent: Spanned<PoolId<HirExpression>>,
        name: SymbolPointer,
        args: Vec<Spanned<PoolId<HirExpression>>>,
    },

    /// A conditional (if) expression.
    ///
    /// Evaluates a condition and executes one of two branches.
    /// Both branches must evaluate to the same type, or one must be void.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let max = if a > b {
    ///     a
    /// } else {
    ///     b
    /// };
    /// ```
    ///
    /// # Fields
    ///
    /// - `condition` — The boolean condition expression
    /// - `then_branch` — Statements in the true branch
    /// - `else_branch` — Optional statements in the false branch
    If {
        /// The condition expression that determines which branch to execute.
        condition: Spanned<PoolId<HirExpression>>,

        /// The statements in the "then" branch (when condition is true).
        then_branch: Vec<Spanned<PoolId<HirStatement>>>,

        /// The optional statements in the "else" branch (when condition is false).
        ///
        /// If `None`, the else branch is empty (equivalent to `{}`).
        else_branch: Option<Vec<Spanned<PoolId<HirStatement>>>>,
    },
    Static {
        id: DeclarationId<HirStaticDeclaration>,
    },
}
