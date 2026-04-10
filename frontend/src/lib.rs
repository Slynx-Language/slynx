pub mod checker;
pub mod hir;
pub mod lexer;
pub mod monomorphizer;
pub mod parser;

// expose the primary type directly; users can still refer to the modules
// through the workspace root if necessary
pub use checker::TypeChecker;
