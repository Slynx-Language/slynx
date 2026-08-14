use common::pool::DedupPool;
use common::{FrontendSymbol, Operator, SymbolsModule};
use slynx_lexer::Lexer;

use crate::ast::GenericIdentifier;
use crate::{ASTExpression, ASTStatement, Parser, Program, Type};

fn parse_program(
    source: &str,
) -> (
    Program,
    SymbolsModule<FrontendSymbol>,
    DedupPool<Type>,
    DedupPool<ASTStatement>,
    DedupPool<ASTExpression>,
) {
    let symbols = SymbolsModule::new();
    let expressions = DedupPool::new();
    let statements = DedupPool::new();
    let types = DedupPool::new();
    let tokens = Lexer::tokenize(source).expect("source should tokenize");
    let program = Parser::new(tokens, &symbols, &expressions, &statements, &types)
        .parse_declarations()
        .expect("source should parse");
    (program, symbols, types, statements, expressions)
}

#[test]
fn generic_function_declaration_maps_params_to_indices() {
    let (program, symbols, types, _, _) = parse_program("func identity<T>(x: T): T { return x; }");

    let func = &program.func()[0];
    assert_eq!(func.type_params.len(), 1);

    let param = func.type_params[0];
    assert_eq!(symbols.get_name(param), "T");

    let arg = &func.args[0].data;
    assert_eq!(symbols.get_name(arg.name.data), "x");
    assert_eq!(types[arg.kind.data], Type::Generic(0));

    assert_eq!(types[func.return_type.data], Type::Generic(0));
}

#[test]
fn generic_function_with_multiple_params() {
    let (program, _, types, _, _) =
        parse_program("func transform<T, T1, T2>(x: T, y: T1, z: T2): T1 { return y; }");

    let func = &program.func()[0];
    assert_eq!(func.type_params.len(), 3);
    assert_eq!(types[func.args[0].data.kind.data], Type::Generic(0));
    assert_eq!(types[func.args[1].data.kind.data], Type::Generic(1));
    assert_eq!(types[func.args[2].data.kind.data], Type::Generic(2));
    assert_eq!(types[func.return_type.data], Type::Generic(1));
}

#[test]
fn non_generic_function_has_no_type_params() {
    let (program, symbols, _, _, _) =
        parse_program("func add(a: int, b: int): int { return a + b; }");

    let func = &program.func()[0];
    assert!(func.type_params.is_empty());

    assert_eq!(symbols.get_name(func.name), "add");
}

#[test]
fn generic_function_without_usage_keeps_scope_clean() {
    // The scope must be popped once the function is parsed, so `T` after the
    // declaration must not resolve to a type parameter.
    let (program, symbols, types, _, _) =
        parse_program("func identity<T>(x: T): T { return x; } func get(): T { return t; }");

    let func = &program.func()[1];
    assert!(func.type_params.is_empty());
    assert_eq!(
        types[func.return_type.data],
        Type::Plain(GenericIdentifier {
            identifier: symbols.intern("T"),
            generic: Default::default(),
        })
    );
}

#[test]
fn generic_call_parses_with_explicit_type_args() {
    let (program, symbols, types, statements, expressions) =
        parse_program("func main(): int { identity<i32>(5); }");

    let func = &program.func()[0];
    let ASTStatement::Expression(expr) = &statements[func.body[0].data] else {
        panic!("expected an expression statement");
    };
    let ASTExpression::FunctionCall { name, args } = &expressions[expr.data] else {
        panic!("expected a function call");
    };
    assert_eq!(args.len(), 1);

    let Type::Plain(GenericIdentifier {
        identifier,
        generic,
    }) = &types[name.data]
    else {
        panic!("expected a generic name");
    };
    assert_eq!(symbols.get_name(*identifier), "identity");
    assert_eq!(generic.len(), 1);
    assert_eq!(
        types[generic[0].data],
        Type::Plain(GenericIdentifier {
            identifier: symbols.intern("i32"),
            generic: Default::default(),
        })
    );

    let ASTExpression::IntLiteral(5) = &expressions[args[0].data] else {
        panic!("expected an integer literal argument");
    };
}

#[test]
fn generic_call_accepts_array_type_args() {
    let (program, symbols, types, statements, expressions) =
        parse_program("func main(): int { funcall<[4]int>(data); }");

    let func = &program.func()[0];
    let ASTStatement::Expression(expr) = &statements[func.body[0].data] else {
        panic!("expected an expression statement");
    };
    let ASTExpression::FunctionCall { name, .. } = &expressions[expr.data] else {
        panic!("expected a function call");
    };
    let Type::Plain(GenericIdentifier {
        identifier,
        generic,
    }) = &types[name.data]
    else {
        panic!("expected a generic name");
    };
    assert_eq!(symbols.get_name(*identifier), "funcall");
    assert_eq!(generic.len(), 1);

    let Type::Array(inner, size) = &types[generic[0].data] else {
        panic!("expected the type argument to be the array type [4]int");
    };
    let Type::Plain(inner) = &types[*inner] else {
        panic!("expected the array inner type to be plain");
    };
    assert_eq!(symbols.get_name(inner.identifier), "int");
    let ASTExpression::IntLiteral(4) = &expressions[*size] else {
        panic!("expected the array size to be the literal 4");
    };
}

#[test]
fn generic_call_inside_generic_body_uses_indices() {
    let (program, symbols, types, statements, expressions) =
        parse_program("func outer<T>(x: T): T { inner<T>(x); }");

    let func = &program.func()[0];
    let ASTStatement::Expression(expr) = &statements[func.body[0].data] else {
        panic!("expected an expression statement");
    };
    let ASTExpression::FunctionCall { name, .. } = &expressions[expr.data] else {
        panic!("expected a function call");
    };
    let Type::Plain(GenericIdentifier {
        identifier,
        generic,
    }) = &types[name.data]
    else {
        panic!("expected a generic name");
    };
    assert_eq!(symbols.get_name(*identifier), "inner");
    assert_eq!(generic.len(), 1);
    assert_eq!(types[generic[0].data], Type::Generic(0));
}

#[test]
fn comparison_operator_still_parses() {
    let (program, _, _, statements, expressions) =
        parse_program("func main(): bool { let a: bool = x < y; }");

    let func = &program.func()[0];
    let ASTStatement::Var { rhs, .. } = &statements[func.body[0].data] else {
        panic!("expected a variable declaration");
    };
    let ASTExpression::Binary { op, .. } = &expressions[rhs.data] else {
        panic!("expected a binary expression");
    };
    assert_eq!(*op, Operator::LessThan);
}
