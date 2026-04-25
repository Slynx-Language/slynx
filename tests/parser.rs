use common::ast::ASTDeclarationKind;
use frontend::{lexer::Lexer, parser::Parser};

#[test]
fn test_parser_parses_main_from_fixture() {
    let source = std::fs::read_to_string("slynx/component.slynx")
        .expect("fixture slynx/component.slynx should exist");
    let tokens = Lexer::tokenize(&source).expect("fixture should tokenize");
    let declarations = Parser::new(tokens)
        .parse_declarations()
        .expect("fixture should parse");

    assert!(
        !declarations.is_empty(),
        "parsed declarations should not be empty"
    );

    let has_main = declarations.iter().any(|decl| {
        matches!(
            &decl.kind,
            ASTDeclarationKind::FuncDeclaration { name, .. } if name.identifier == "main"
        )
    });

    assert!(has_main, "expected fixture to contain func main");
}
