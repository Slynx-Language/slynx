//! Boundaries for AST lookup and lowering into semantic HIR data.

pub mod lookup;
pub mod lowerer;
pub use lowerer::ASTLowerer;
pub mod typing;
