use std::path::PathBuf;

use crate::{
    backend::compiler::{js::WebCompiler, slynx_compiler::SlynxCompiler},
    frontend::checker::TypeChecker,
    middleend::hir::SlynxHir,
    middleend::intermediate::IntermediateRepr,
};

pub mod backend;
pub mod frontend;
pub mod middleend;

mod context;

pub use context::*;

pub fn compile_code(path: PathBuf) -> color_eyre::eyre::Result<()> {
    let code = std::fs::read_to_string(&path)?;
    let tokens = frontend::parser::lexer::Lexer::tokenize(&code)?;
    let mut ast = frontend::parser::Parser::new(tokens);
    let decls = ast.parse_declarations()?;
    let mut hir = SlynxHir::new();

    hir.generate(decls)?;
    let module = TypeChecker::check(&mut hir)?;
    let mut intermediate = IntermediateRepr::new();
    intermediate.generate(hir.declarations, module);
    let compiler = WebCompiler::new();
    let code = compiler.compile(intermediate);
    std::fs::write(path.with_extension("js"), code)?;
    Ok(())
}
