use common::ast::{
    ASTDeclaration, ASTDeclarationKind, ASTExpressionKind, ASTStatement, ASTStatementKind, Operator,
};
use frontend::{lexer::Lexer, parser::Parser};

fn parse_source(source: &str) -> Vec<ASTDeclaration> {
    let tokens = Lexer::tokenize(source).expect("source should tokenize");
    Parser::new(tokens)
        .parse_declarations()
        .expect("source should parse")
}

fn parse_source_error(source: &str) -> String {
    let tokens = Lexer::tokenize(source).expect("source should tokenize");
    Parser::new(tokens)
        .parse_declarations()
        .expect_err("source should fail to parse")
        .to_string()
}

fn function_body<'a>(declarations: &'a [ASTDeclaration], name: &str) -> &'a [ASTStatement] {
    let declaration = declarations
        .iter()
        .find(|decl| matches!(&decl.kind, ASTDeclarationKind::FuncDeclaration { name: decl_name, .. } if decl_name.identifier == name))
        .expect("function declaration should exist");

    let ASTDeclarationKind::FuncDeclaration { body, .. } = &declaration.kind else {
        panic!("expected function declaration");
    };

    body.as_slice()
}
#[test]
fn parses_function_body_tuple() {
    let declarations = parse_source(
        r#"
func main(): int {
    let value = (1, (1,2,3));
    value
}
"#,
    );

    let body = function_body(&declarations, "main");
    assert_eq!(body.len(), 2);

    let ASTStatementKind::Var { name, ty, rhs } = &body[0].kind else {
        panic!("expected let statement");
    };
    assert_eq!(name, "value");
    assert!(ty.is_none());
    assert!(matches!(rhs.kind, ASTExpressionKind::Tuple(_)));
}
#[test]
fn parses_function_body_statement_shapes() {
    let declarations = parse_source(
        r#"
func main(): int {
    let value = 1;
    let mut total: int = 2;
    total = value + total;
    total
}
"#,
    );

    let body = function_body(&declarations, "main");
    assert_eq!(body.len(), 4);

    let ASTStatementKind::Var { name, ty, rhs } = &body[0].kind else {
        panic!("expected let statement");
    };
    assert_eq!(name, "value");
    assert!(ty.is_none());
    assert!(matches!(rhs.kind, ASTExpressionKind::IntLiteral(1)));

    let ASTStatementKind::MutableVar { name, ty, rhs } = &body[1].kind else {
        panic!("expected mutable let statement");
    };
    assert_eq!(name, "total");
    assert_eq!(
        ty.as_ref()
            .expect("typed let should keep its type")
            .identifier,
        "int"
    );
    assert!(matches!(rhs.kind, ASTExpressionKind::IntLiteral(2)));

    let ASTStatementKind::Assign { lhs, rhs } = &body[2].kind else {
        panic!("expected assign statement");
    };
    assert!(matches!(&lhs.kind, ASTExpressionKind::Identifier(name) if name == "total"));
    let ASTExpressionKind::Binary {
        lhs: binary_lhs,
        op,
        rhs: binary_rhs,
    } = &rhs.kind
    else {
        panic!("expected binary expression on assign rhs");
    };
    assert!(matches!(&binary_lhs.kind, ASTExpressionKind::Identifier(name) if name == "value"));
    assert!(matches!(&binary_rhs.kind, ASTExpressionKind::Identifier(name) if name == "total"));
    assert!(matches!(op, Operator::Add));

    let ASTStatementKind::Expression(expr) = &body[3].kind else {
        panic!("expected trailing expression statement");
    };
    assert!(matches!(&expr.kind, ASTExpressionKind::Identifier(name) if name == "total"));
}

#[test]
fn parses_arrow_function_body_as_single_expression_statement() {
    let declarations = parse_source(
        r#"
func main(arg: int): int -> sum(arg, 2);
"#,
    );

    let body = function_body(&declarations, "main");
    assert_eq!(body.len(), 1);

    let ASTStatementKind::Expression(expr) = &body[0].kind else {
        panic!("expected expression body");
    };
    let ASTExpressionKind::FunctionCall { name, args } = &expr.kind else {
        panic!("expected function call");
    };

    assert_eq!(name.identifier, "sum");
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0].kind, ASTExpressionKind::Identifier(arg) if arg == "arg"));
    assert!(matches!(args[1].kind, ASTExpressionKind::IntLiteral(2)));
}

#[test]
fn parses_function_call_without_arguments() {
    let declarations = parse_source(
        r#"
func main(): int {
    noop();
    0
}
"#,
    );

    let body = function_body(&declarations, "main");
    assert_eq!(body.len(), 2);

    let ASTStatementKind::Expression(expr) = &body[0].kind else {
        panic!("expected function call statement");
    };
    let ASTExpressionKind::FunctionCall { name, args } = &expr.kind else {
        panic!("expected function call");
    };

    assert_eq!(name.identifier, "noop");
    assert!(
        args.is_empty(),
        "zero-argument calls should parse with no args"
    );
}

#[test]
fn parses_object_expression_and_field_assignment() {
    let declarations = parse_source(
        r#"
object Pessoa{
    name: str,
    age: int
}

func main(): int {
    let mut person = Pessoa(name: "John", age: 10);
    person.age = 55;
    person.age
}
"#,
    );

    let body = function_body(&declarations, "main");
    assert_eq!(body.len(), 3);

    let ASTStatementKind::MutableVar { rhs, .. } = &body[0].kind else {
        panic!("expected mutable let statement");
    };
    let ASTExpressionKind::ObjectExpression { name, fields } = &rhs.kind else {
        panic!("expected object expression");
    };
    assert_eq!(name.identifier, "Pessoa");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "name");
    assert!(matches!(
        fields[0].expr.kind,
        ASTExpressionKind::StringLiteral(_)
    ));
    assert_eq!(fields[1].name, "age");
    assert!(matches!(
        fields[1].expr.kind,
        ASTExpressionKind::IntLiteral(10)
    ));

    let ASTStatementKind::Assign { lhs, rhs } = &body[1].kind else {
        panic!("expected field assignment");
    };
    let ASTExpressionKind::FieldAccess { parent, field } = &lhs.kind else {
        panic!("expected field access on assign lhs");
    };
    assert_eq!(field, "age");
    assert!(matches!(&parent.kind, ASTExpressionKind::Identifier(name) if name == "person"));
    assert!(matches!(rhs.kind, ASTExpressionKind::IntLiteral(55)));
}

#[test]
fn allows_last_expression_without_trailing_semicolon() {
    let declarations = parse_source(
        r#"
func main(): int {
    let value = 1;
    value
}
"#,
    );

    let body = function_body(&declarations, "main");
    assert_eq!(body.len(), 2);
    assert!(
        matches!(&body[1].kind, ASTStatementKind::Expression(expr) if matches!(&expr.kind, ASTExpressionKind::Identifier(name) if name == "value"))
    );
}

#[test]
fn rejects_missing_semicolon_between_statements() {
    let error = parse_source_error(
        r#"
func main(): int {
    let value = 1
    let other = 2;
    other
}
"#,
    );

    assert!(error.contains("Unexpected token"));
    assert!(error.contains("'let'"));
}

#[test]
fn parses_trailing_dot_float_literal() {
    let declarations = parse_source(
        r#"
func main(): float {
    let value = 0.;
    value
}
"#,
    );

    let body = function_body(&declarations, "main");
    assert_eq!(body.len(), 2);

    let ASTStatementKind::Var { rhs, .. } = &body[0].kind else {
        panic!("expected let statement");
    };

    assert!(matches!(rhs.kind, ASTExpressionKind::FloatLiteral(value) if value == 0.0));
}
