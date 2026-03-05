pub mod checker;
pub mod parser;

// expose the primary type directly; users can still refer to the modules
// through the workspace root if necessary
pub use checker::TypeChecker;  
