use std::{path::PathBuf, sync::Arc};

mod compilation_context;
pub use compilation_context::*;

pub use slynx_hir;
pub use slynx_ir;
use slynx_ir::SlynxIR;
pub use slynx_lexer;
pub use slynx_monomorphizer;
pub use slynx_parser;
pub use slynx_typechecker;

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
