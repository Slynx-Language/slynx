use std::{path::PathBuf, sync::Arc};

pub mod context;
pub use context::*;
pub use frontend::checker;
pub use frontend::parser;


pub fn compile_code(path: PathBuf) -> color_eyre::eyre::Result<()> {
    let context = SlynxContext::new(Arc::new(path))?;
    let output = context.compile()?;
    output.write()?;
    Ok(())
}
