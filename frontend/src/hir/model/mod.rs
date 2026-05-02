//! HIR Model Types
//!
//! This module defines the core data structures that represent the High-Level
//! Intermediate Representation (HIR) of the program.
//!
//! # Organization
//!
//! The module is divided into four submodules, each representing a different
//! category of HIR elements:
//!
//! - **[`declarations`]** — Top-level program declarations (functions, components, objects, aliases)
//! - **[`expression`]** — Expression nodes in the HIR
//! - **[`statements`]** — Statement nodes in the HIR
//! - **[`types`]** — Type representations in the HIR type system
//!
//! # Design Philosophy
//!
//! The HIR model follows these principles:
//!
//! 1. **Explicit Types** — Every expression and declaration has an associated [`TypeId`]
//! 2. **Unique Identifiers** — All significant elements have unique IDs for tracking
//! 3. **Span Preservation** — Source location information is retained for diagnostics
//! 4. **Recursive Structure** — Expressions and statements can nest arbitrarily
//!
//! # Key Types
//!
//! - [`HirDeclaration`] — A top-level declaration (function, component, etc.)
//! - [`HirExpression`] — An expression node in the HIR
//! - [`HirStatement`] — A statement node in the HIR
//! - [`HirType`] — A type in the HIR type system
//!
//! # Example
//!
//! ```rust
//! # use slynx_frontend::hir::model::*;
//! # use common::Span;
//! # let span = Span::default();
//!
//! // A function declaration
//! let func_decl = HirDeclaration::new_function(
//!     vec![],           // statements
//!     vec![],           // arguments
//!     "my_func".into(), // name
//!     span,
//!     DeclarationId::new(),
//!     TypeId::from_raw(0),
//! );
//!
//! // An integer literal expression
//! let int_expr = HirExpression {
//!     id: ExpressionId::new(),
//!     ty: TypeId::from_raw(0), // int type
//!     kind: HirExpressionKind::Int(42),
//!     span,
//! };
//! ```

mod declarations;
mod expression;
mod statements;
mod types;

pub use declarations::*;
pub use expression::*;
pub use statements::*;
pub use types::*;
