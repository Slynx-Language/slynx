use std::{path::PathBuf, sync::Arc};

mod compilation_context;
pub use compilation_context::*;
pub use frontend::checker;
pub use frontend::hir;
pub use frontend::lexer;
pub use frontend::parser;

pub use middleend;
use middleend::*;

///A trait to define how to compile the Slynx IR
pub trait SlynxCompiler {
    ///The type for Expressions
    type ExpressionType;
    ///The type for Statments
    type StatementType;

    ///Compiles the provided `instructions`. The `ctx` and `ir` are the context of the IR and the IR itself. The provided `handle`
    ///is the handle of the current compiling context
    fn compile_instructions(
        &mut self,
        instructions: &[middleend::Instruction],
        ctx: IRPointer<Context>,
        ir: &SlynxIR,
    ) -> Vec<Self::StatementType>;

    ///Compiles the provided `expr` using the given `ctx` and `ir`. The provided `handle` is the handle
    ///of the context of compiler itself, instead of the IR
    fn compile_expression(
        &mut self,
        expr: IRPointer<Value, 1>,
        ctx: IRPointer<Context>,
        ir: &SlynxIR,
    ) -> Self::ExpressionType;
    ///Starts compiling the provided `ir` and returns its result in bytes
    fn compile(self, ir: SlynxIR) -> Vec<u8>;
}

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
