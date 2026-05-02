use std::{path::PathBuf, sync::Arc};

mod compilation_context;
pub use compilation_context::*;
pub use frontend::checker;
pub use frontend::hir;
pub use frontend::lexer;
pub use frontend::parser;

pub use middleend;
use middleend::*;

///Compiels the provided `slynx` code from the provided `path` and writes the slynx IR textual form into the same `path` but with extension `sir`
pub fn compile_code(path: PathBuf) -> color_eyre::eyre::Result<()> {
    let context = SlynxContext::new(Arc::new(path))?;
    let output = context.compile()?;
    output.write()?;
    Ok(())
}

///Compiels the provided `slynx` code from the provided `path` and returns the compiled slynx IR
pub fn compile_to_ir(path: PathBuf) -> color_eyre::eyre::Result<SlynxIR> {
    let context = SlynxContext::new(Arc::new(path))?;
    let output = context.compile()?;
    Ok(output.ir())
}
