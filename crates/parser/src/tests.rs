use common::pool::{DedupPool, PoolId};
use common::{FrontendSymbol, Operator, SymbolsModule};
use slynx_lexer::Lexer;
use smallvec::smallvec;

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

    let func = &program.func().get(PoolId::new(0));
    assert_eq!(func.generics.type_params.len(), 1);

    let param = func.generics.type_params[0];
    assert_eq!(symbols.get_name(param), "T");

    let arg = &func.args[0].data;
    let generic_type = Type::Plain(GenericIdentifier {
        generic: smallvec![],
        identifier: symbols.intern("T"),
    });
    assert_eq!(symbols.get_name(arg.name.data), "x");
    assert_eq!(types[arg.kind.data], generic_type);

    assert_eq!(types[func.return_type.data], generic_type);
}

#[test]
fn generic_function_with_multiple_params() {
    let (program, symbols, types, _, _) =
        parse_program("func transform<T, T1, T2>(x: T, y: T1, z: T2): T1 { return y; }");

    let func = &program.func().get(PoolId::new(0));
    assert_eq!(func.generics.type_params.len(), 3);
    assert_eq!(
        types[func.args[0].data.kind.data],
        Type::Plain(GenericIdentifier {
            generic: smallvec![],
            identifier: symbols.intern("T")
        })
    );
    assert_eq!(
        types[func.args[1].data.kind.data],
        Type::Plain(GenericIdentifier {
            generic: smallvec![],
            identifier: symbols.intern("T1")
        })
    );
    assert_eq!(
        types[func.args[2].data.kind.data],
        Type::Plain(GenericIdentifier {
            generic: smallvec![],
            identifier: symbols.intern("T2")
        })
    );
    assert_eq!(
        types[func.return_type.data],
        Type::Plain(GenericIdentifier {
            generic: smallvec![],
            identifier: symbols.intern("T1")
        })
    );
}

#[test]
fn non_generic_function_has_no_type_params() {
    let (program, symbols, _, _, _) =
        parse_program("func add(a: int, b: int): int { return a + b; }");

    let func = &program.func().get(PoolId::new(0));
    assert!(func.generics.type_params.is_empty());

    assert_eq!(symbols.get_name(func.name), "add");
}

#[test]
fn generic_function_without_usage_keeps_scope_clean() {
    // The scope must be popped once the function is parsed, so `T` after the
    // declaration must not resolve to a type parameter.
    let (program, symbols, types, _, _) =
        parse_program("func identity<T>(x: T): T { return x; } func get(): T { return t; }");

    let func = &program.func().get(PoolId::new(1));
    assert!(func.generics.type_params.is_empty());
    assert_eq!(
        types[func.return_type.data],
        Type::Plain(GenericIdentifier {
            identifier: symbols.intern("T"),
            generic: Default::default(),
        })
    );
}

///Every declaration that can carry an interface implementation list and a
///`where` clause list must parse both, and the list parser must not swallow the
///`{` that opens the body.
#[test]
fn interface_lists_and_clauses_parse_before_the_body() {
    let sources = [
        "interface I { func m(&self) -> str; }",
        "interface I where T: I { func m(&self) -> str; }",
        "interface J: I where T: I { func m(&self) -> str; }",
        "interface J: I, I where T: I, U: I { func m(&self) -> str; }",
        "object O { f: int } extend O: I where T: I { func m(&self) -> str { \"\" } }",
        "enum E { A } extend E: I where T: I { func m(&self) -> str { \"\" } }",
    ];

    for source in sources {
        parse_program(source);
    }
}

///Within a `where` clause list the bounds of one clause are separated by `&`
///and the clauses themselves by `,`. Neither list may demand a separator after
///its last item.
#[test]
fn nested_bounds_do_not_require_a_separator_after_the_last_one() {
    let (program, _, types, _, _) =
        parse_program("interface J where T: I & K, U: I { func m(&self) -> str; }");

    let interface = program.interfaces().iter().next().expect("one interface");
    let clauses = &interface.generics.clauses;
    assert_eq!(clauses.len(), 2);
    assert_eq!(clauses[0].data.bounds.len(), 2);
    assert_eq!(clauses[1].data.bounds.len(), 1);
    // The `&`-separated bounds are `I` and `K`, not one `I & K` type.
    assert!(matches!(
        types[clauses[0].data.bounds[0].data],
        Type::Plain(_)
    ));
    assert!(matches!(
        types[clauses[0].data.bounds[1].data],
        Type::Plain(_)
    ));
}

///The `where` clause list of a function must be read after the return type and
///before the token that opens its body, otherwise the clause list is read as
///part of the body.
#[test]
fn where_clause_follows_the_return_type_and_precedes_the_body() {
    let (program, _, _, _, _) = parse_program("func m<T>(x: T) -> T where T: int { return x; }");

    let func = program.func().iter().next().expect("one function");
    assert_eq!(func.generics.clauses.len(), 1);
    assert_eq!(func.body.len(), 1);
}

///Fields and methods may be mixed in an object body; only two adjacent fields
///need a `,` between them.
#[test]
fn object_fields_and_methods_can_be_mixed() {
    let (program, _, _, _, _) = parse_program(
        "object O { a: int, b: int, func m(&self) -> int { 1 } func n(&self) -> int { 2 } }",
    );

    let object = program.object().iter().next().expect("one object");
    assert_eq!(object.fields.len(), 2);
    assert_eq!(object.methods.len(), 2);
}

#[test]
fn object_fields_still_need_a_separator() {
    let tokens = Lexer::tokenize("object O { a: int b: int }").expect("source should tokenize");
    let symbols = SymbolsModule::new();
    let types = DedupPool::new();
    let error = Parser::new(
        tokens,
        &symbols,
        &DedupPool::new(),
        &DedupPool::new(),
        &types,
    )
    .parse_declarations()
    .expect_err("two adjacent fields must not parse");
    assert!(
        error.to_string().contains("','"),
        "error should name the missing separator, got: {error}"
    );
}

///`func` inside an enum body declares a method, not a variant.
#[test]
fn enum_body_accepts_methods() {
    let (program, symbols, _, _, _) = parse_program("enum E { A, func m(&self) -> int { 1 } }");

    let enumeration = program.enums().iter().next().expect("one enum");
    assert_eq!(enumeration.variants.len(), 1);
    assert_eq!(symbols.get_name(enumeration.variants[0].name.data), "A");
    assert_eq!(enumeration.methods.len(), 1);
    assert_eq!(symbols.get_name(enumeration.methods[0].method_name), "m");
}

///A stylesheet header is `name(args) uses X, Y: Interfaces where T: U {`, so the
///interface list comes after the arguments and the `uses` list.
#[test]
fn stylesheet_interface_list_follows_arguments_and_uses() {
    let sources = [
        "stylesheet S(color: int) { styles { default { backgroundColor: color } } }",
        "stylesheet S(color: int): I { styles { default { backgroundColor: color } } }",
        "stylesheet S(color: int) uses P() { styles { default { backgroundColor: color } } }",
    ];

    for source in sources {
        parse_program(source);
    }
}

#[test]
fn interface_has_no_super_interface_list() {
    let (program, _, _, _, _) = parse_program("interface I { func m(&self) -> str; }");

    let interface = program.interfaces().iter().next().expect("one interface");
    assert!(interface.super_interfaces.is_empty());
}

#[test]
fn generic_call_parses_with_explicit_type_args() {
    let (program, symbols, types, statements, expressions) =
        parse_program("func main(): int { identity<i32>(5); }");

    let func = &program.func().get(PoolId::new(0));
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

    let func = &program.func().get(PoolId::new(0));
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

    let func = &program.func().get(PoolId::new(0));
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
    assert_eq!(
        types[generic[0].data],
        Type::Plain(GenericIdentifier {
            generic: smallvec![],
            identifier: symbols.intern("T"),
        })
    );
}

#[test]
fn comparison_operator_still_parses() {
    let (program, _, _, statements, expressions) =
        parse_program("func main(): bool { let a: bool = x < y; }");

    let func = &program.func().get(PoolId::new(0));
    let ASTStatement::Var { rhs, .. } = &statements[func.body[0].data] else {
        panic!("expected a variable declaration");
    };
    let ASTExpression::Binary { op, .. } = &expressions[rhs.data] else {
        panic!("expected a binary expression");
    };
    assert_eq!(*op, Operator::LessThan);
}

#[test]
fn right_shift_parses_without_panicking() {
    // `>>` is not a single lexer token: the parser must recognise it as two
    // consecutive `>` tokens instead of falling into an unreachable!().
    let (program, symbols, _, statements, expressions) =
        parse_program("func main(): int { let a: int = x >> y; }");

    let func = &program.func().get(PoolId::new(0));
    let ASTStatement::Var { rhs, .. } = &statements[func.body[0].data] else {
        panic!("expected a variable declaration");
    };
    let ASTExpression::Binary {
        op,
        lhs,
        rhs: shift_count,
    } = &expressions[rhs.data]
    else {
        panic!("expected a binary expression");
    };
    assert_eq!(*op, Operator::RightShift);
    let ASTExpression::Identifier(lhs_name) = &expressions[lhs.data] else {
        panic!("expected the shift target to be an identifier");
    };
    let ASTExpression::Identifier(rhs_name) = &expressions[shift_count.data] else {
        panic!("expected the shift amount to be an identifier");
    };
    assert_eq!(*lhs_name, symbols.intern("x"));
    assert_eq!(*rhs_name, symbols.intern("y"));
}

#[test]
fn chained_right_shift_parses_without_panicking() {
    // After consuming the two `>` tokens of the first `>>`, the loop must
    // re-check the *next* token pair instead of misreading a lone `>`.
    let (program, symbols, _, statements, expressions) =
        parse_program("func main(): int { let a: int = x >> y >> z; }");

    let func = &program.func().get(PoolId::new(0));
    let ASTStatement::Var { rhs, .. } = &statements[func.body[0].data] else {
        panic!("expected a variable declaration");
    };
    let outer = &expressions[rhs.data];
    let ASTExpression::Binary { op, lhs, rhs: rest } = outer else {
        panic!("expected an outer binary expression");
    };
    assert_eq!(*op, Operator::RightShift);
    let ASTExpression::Identifier(rhs_is_x) = &expressions[lhs.data] else {
        panic!("expected the first operand to be an identifier");
    };
    assert_eq!(*rhs_is_x, symbols.intern("x"));
    let ASTExpression::Binary {
        op, lhs, rhs: last, ..
    } = &expressions[rest.data]
    else {
        panic!("expected the tail to be another shift expression");
    };
    assert_eq!(*op, Operator::RightShift);
    let ASTExpression::Identifier(rhs_is_y) = &expressions[lhs.data] else {
        panic!("expected the second operand to be an identifier");
    };
    let ASTExpression::Identifier(rhs_is_z) = &expressions[last.data] else {
        panic!("expected the third operand to be an identifier");
    };
    assert_eq!(*rhs_is_y, symbols.intern("y"));
    assert_eq!(*rhs_is_z, symbols.intern("z"));
}
