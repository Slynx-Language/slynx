use frontend::hir::{
    SlynxHir,
    model::{HirDeclarationKind, HirExpression, HirExpressionKind, HirStatementKind},
};
use frontend::lexer::Lexer;
use frontend::parser::Parser;
use slynx::hir::model::HirDeclaration;

pub fn load_hir(path: &str) -> SlynxHir {
    let source = std::fs::read_to_string(path).expect("source file should exist");
    let tokens = Lexer::tokenize(&source).expect("source should tokenize");
    let declarations = Parser::new(tokens)
        .parse_declarations()
        .expect("source should parse");
    let mut hir = SlynxHir::new();
    hir.generate(declarations).expect("HIR should generate");
    hir
}

pub fn find_main_call_args(hir: &mut SlynxHir) -> Option<&mut Vec<HirExpression>> {
    let pos = hir.declarations.iter().position(|v| match v.kind {
        HirDeclarationKind::Function { name, .. } if hir.get_name(name) == "main" => true,
        _ => false,
    })?;
    let HirDeclaration {
        kind: HirDeclarationKind::Function { statements, .. },
        ..
    } = &mut hir.declarations[pos]
    else {
        unreachable!()
    };
    for statement in statements {
        let expr = match &mut statement.kind {
            HirStatementKind::Variable { value, .. } => value,
            HirStatementKind::Expression { expr } => expr,
            HirStatementKind::Return { expr } => expr,
            HirStatementKind::Assign { value, .. } => value,
            HirStatementKind::While { .. } => continue,
        };
        let HirExpressionKind::FunctionCall { args, .. } = &mut expr.kind else {
            continue;
        };
        return Some(args);
    }

    None
}
