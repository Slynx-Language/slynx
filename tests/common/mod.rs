#![allow(dead_code)]

use slynx_hir::{
    HIRError, SlynxHir,
    model::{HirDeclarationKind, HirExpression, HirExpressionKind, HirStatementKind},
};
use slynx_lexer::Lexer;
use slynx_parser::Parser;
pub fn load_source(source: &str) -> Result<slynx_hir::SlynxHir, HIRError> {
    let tokens = slynx_lexer::Lexer::tokenize(source).expect("source should tokenize");
    let declarations = slynx_parser::Parser::new(tokens)
        .parse_declarations()
        .expect("source should parse");
    let mut hir = slynx_hir::SlynxHir::new();
    let mut modules = Vec::new();
    // create a single source node with file_id 0
    modules.push(slynx_hir::module_loader::SourceNode::new(
        slynx_hir::module_loader::FileId::from_raw(0),
        declarations,
    ));
    hir.generate(&modules)?;
    Ok(hir)
}
pub fn load_hir(path: &str) -> SlynxHir {
    let source = std::fs::read_to_string(path).expect("source file should exist");
    let tokens = Lexer::tokenize(&source).expect("source should tokenize");
    let declarations = Parser::new(tokens)
        .parse_declarations()
        .expect("source should parse");
    let mut hir = SlynxHir::new();
    let modules = vec![slynx_hir::module_loader::SourceNode::new(
        slynx_hir::module_loader::FileId::from_raw(0),
        declarations,
    )];
    hir.generate(&modules).expect("HIR should generate");
    hir
}

pub fn find_main_call_args(hir: &SlynxHir) -> Option<&Vec<HirExpression>> {
    let main_name = hir.intern_name("main");
    let statements = {
        let mut stmts = None;
        'out: for file in &hir.files {
            for decl in file.declarations() {
                if let HirDeclarationKind::Function {
                    name,
                    ref statements,
                    ..
                } = decl.1.kind
                    && name == main_name
                {
                    stmts = Some(statements);
                    break 'out;
                }
            }
        }
        stmts
    }?;
    for statement in statements {
        let expr = match &statement.kind {
            HirStatementKind::Variable { value, .. } => value,
            HirStatementKind::Expression { expr } => expr,
            HirStatementKind::Return { expr } => expr,
            HirStatementKind::Assign { value, .. } => value,
            HirStatementKind::While { .. } => continue,
        };
        let HirExpressionKind::FunctionCall { args, .. } = &expr.kind else {
            continue;
        };
        return Some(args);
    }

    None
}
