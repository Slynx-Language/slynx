use std::path::PathBuf;

mod compilation_context;

pub use common;
pub use compilation_context::*;
pub use slynx_hir;
pub use slynx_ir;
use slynx_ir::SlynxIR;
pub use slynx_lexer;
pub use slynx_monomorphizer;
pub use slynx_parser;

///Compiles the provided `slynx` code from the provided `path` and returns the compiled slynx IR
pub fn compile_to_ir(path: PathBuf, std: Option<PathBuf>) -> color_eyre::Result<SlynxIR> {
    let context = SlynxContext::new(path, std)?;
    let output = context.compile()?;
    Ok(output.ir())
}
