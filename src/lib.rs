use std::path::PathBuf;

pub mod context;
use backend::js::WebCompiler;
pub use context::*;
use backend::slynx_compiler::SlynxCompiler;
use frontend::TypeChecker;
pub use frontend::checker;
pub use frontend::parser;
pub use backend::compiler;
use middleend::IntermediateRepr;
use middleend::SlynxHir;
pub use middleend::hir;
pub use middleend::intermediate;

pub fn compile_code(path: PathBuf) -> color_eyre::eyre::Result<()> {
    let code = std::fs::read_to_string(&path)?;
    let tokens = parser::lexer::Lexer::tokenize(&code)?;
    let mut ast = parser::Parser::new(tokens);
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
