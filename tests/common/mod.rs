use frontend::hir::{
    SlynxHir,
    definitions::{HirDeclarationKind, HirExpression, HirExpressionKind, HirStatementKind},
};
use frontend::lexer::Lexer;
use frontend::parser::Parser;

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
    for declaration in &mut hir.declarations {
        let HirDeclarationKind::Function {
            name, statements, ..
        } = &mut declaration.kind
        else {
            continue;
        };
        if name != "main" {
            continue;
        }

        for statement in statements {
            let expr = match &mut statement.kind {
                HirStatementKind::Variable { value, .. } => value,
                HirStatementKind::Expression { expr } => expr,
                HirStatementKind::Return { expr } => expr,
                HirStatementKind::Assign { value, .. } => value,
                HirStatementKind::If { condition, .. } => condition,
            };
            let HirExpressionKind::FunctionCall { args, .. } = &mut expr.kind else {
                continue;
            };
            return Some(args);
        }
    }
    None
}
