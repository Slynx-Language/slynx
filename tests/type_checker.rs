mod common;

use slynx_hir::{
    ExpressionId,
    model::{
        HirDeclaration, HirDeclarationKind, HirExpression, HirExpressionKind, HirStatementKind,
    },
};
use slynx_typechecker::{TypeChecker, error::TypeErrorKind};

fn load_hir(source: &str) -> slynx_hir::SlynxHir {
    let tokens = slynx_lexer::Lexer::tokenize(source).expect("source should tokenize");
    let declarations = slynx_parser::Parser::new(tokens)
        .parse_declarations()
        .expect("source should parse");
    let mut hir = slynx_hir::SlynxHir::new();
    hir.generate(&declarations).expect("HIR should generate");
    hir
}

fn find_main_call_args(hir: &mut slynx_hir::SlynxHir) -> Option<&mut Vec<HirExpression>> {
    let pos = hir.declarations.iter().position(|v| {
        matches!(
            v.kind,
            HirDeclarationKind::Function { name, .. } if hir.get_name(name) == "main"
        )
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

// ─── Function call tests ───────────────────────────────────────────

#[test]
fn function_calls_work_with_mixed_declaration_order() {
    let mut hir = load_hir("func bar(): void {} func main(): void { bar() }");
    TypeChecker::check(&mut hir).expect("function call should resolve with declaration ids");
}

#[test]
fn rejects_function_call_with_extra_arg() {
    let mut hir =
        load_hir("func add(a: int, b: int): int { a + b } func main(): void { add(1, 2) }");
    let args = find_main_call_args(&mut hir).expect("expected to find a function call in main");
    let template = args.first().expect("call should have at least one arg");
    args.push(HirExpression {
        id: ExpressionId::new(),
        ty: template.ty,
        kind: HirExpressionKind::Int(0),
        span: template.span,
    });

    let err = TypeChecker::check(&mut hir)
        .expect_err("type checker should reject function calls with extra args");

    match &err.kind {
        TypeErrorKind::InvalidFuncallArgLength {
            expected_length,
            received_length,
        } => {
            assert_eq!(*expected_length, 2);
            assert_eq!(*received_length, 3);
        }
        other => panic!("expected InvalidFuncallArgLength, got {other:?}"),
    }
}

#[test]
fn rejects_function_call_with_missing_arg() {
    let mut hir =
        load_hir("func add(a: int, b: int): int { a + b } func main(): void { add(1, 2) }");
    let args = find_main_call_args(&mut hir).expect("expected to find a function call in main");
    args.pop();

    let err = TypeChecker::check(&mut hir)
        .expect_err("type checker should reject function calls with missing args");

    match &err.kind {
        TypeErrorKind::InvalidFuncallArgLength {
            expected_length,
            received_length,
        } => {
            assert_eq!(*expected_length, 2);
            assert_eq!(*received_length, 1);
        }
        other => panic!("expected InvalidFuncallArgLength, got {other:?}"),
    }
}

#[test]
fn rejects_function_call_with_wrong_argument_type() {
    let mut hir =
        load_hir("func takes_int(value: int): void {} func main(): void { takes_int(42) }");
    let bool_ty = hir.bool_type();
    let args = find_main_call_args(&mut hir).expect("expected to find a function call in main");
    let first_arg = args.first_mut().expect("call should have at least one arg");
    first_arg.kind = HirExpressionKind::Bool(true);
    first_arg.ty = bool_ty;

    let err = TypeChecker::check(&mut hir)
        .expect_err("type checker should reject function calls with wrong arg type");

    assert!(
        matches!(err.kind, TypeErrorKind::IncompatibleTypes { .. }),
        "expected IncompatibleTypes, got {:?}",
        err.kind
    );
}

// ─── Return / control flow tests ───────────────────────────────────

#[test]
fn rejects_function_without_return_value_for_non_void_return_type() {
    let mut hir = load_hir("func main(): int { let x = 12; }");

    let err = TypeChecker::check(&mut hir).expect_err("non-void functions must return a value");

    match &err.kind {
        TypeErrorKind::MissingReturnValue { expected } => {
            assert!(
                matches!(expected, slynx_hir::model::HirType::Int),
                "expected missing int return, got {expected:?}"
            );
        }
        other => panic!("expected MissingReturnValue, got {other:?}"),
    }
}

#[test]
fn preserves_non_expression_tail_statement_in_function_body() {
    let hir = load_hir("func main(): void { let x = 12; }");
    let main_symbol = hir
        .modules
        .retrieve_symbol("main")
        .expect("main symbol should exist");

    let main_fn = hir
        .declarations
        .iter()
        .find(|declaration| {
            matches!(
                declaration.kind,
                HirDeclarationKind::Function { ref name, .. } if *name == main_symbol
            )
        })
        .expect("main function should exist");

    let HirDeclarationKind::Function { statements, .. } = &main_fn.kind else {
        unreachable!();
    };

    assert_eq!(
        statements.len(),
        1,
        "last non-expression statement should be preserved"
    );
    assert!(
        matches!(statements[0].kind, HirStatementKind::Variable { .. }),
        "expected trailing let statement to stay in the body"
    );
}

#[test]
fn rejects_while_with_non_boolean_condition() {
    let mut hir = load_hir("func main(): void { while 10 { 0; } }");

    let err = TypeChecker::check(&mut hir).expect_err("while conditions should require bool");

    assert!(
        matches!(err.kind, TypeErrorKind::IncompatibleTypes { .. }),
        "expected IncompatibleTypes, got {:?}",
        err.kind
    );
}

#[test]
fn rejects_invalid_statement_inside_while_body() {
    let mut hir = load_hir(
        "func takes_int(value: int): void {} func main(): void { while true { takes_int(false); } }",
    );

    let err =
        TypeChecker::check(&mut hir).expect_err("while body statements should be type-checked");

    assert!(
        matches!(err.kind, TypeErrorKind::IncompatibleTypes { .. }),
        "expected IncompatibleTypes, got {:?}",
        err.kind
    );
}

// ─── Field / tuple access tests ────────────────────────────────────

#[test]
fn resolves_field_access_for_variables_typed_via_alias() {
    let mut hir = load_hir(
        "object Person { age: int } alias PersonAlias = Person;
         func make_person(): PersonAlias { Person(age: 22) }
         func main(): int { let person = make_person(); person.age }",
    );
    TypeChecker::check(&mut hir).expect("field access should resolve through aliases");
}

#[test]
fn resolves_tuple_access_for_tuple_variables() {
    let mut hir = load_hir("func main(): int { let pair = (10, 20); pair.0 }");
    TypeChecker::check(&mut hir).expect("tuple access should resolve through the checker");
}

#[test]
fn resolves_named_field_access_after_tuple_access() {
    let mut hir = load_hir(
        "object Person { age: int }
         func main(): int { let pair = (Person(age: 22), \"ok\"); pair.0.age }",
    );
    TypeChecker::check(&mut hir)
        .expect("named field access after tuple access should resolve cleanly");
}

#[test]
fn rejects_tuple_access_with_invalid_index() {
    let mut hir = load_hir("func main(): int { let pair = (10, 20); pair.2 }");

    let err =
        TypeChecker::check(&mut hir).expect_err("tuple accesses should reject invalid indexes");

    match &err.kind {
        TypeErrorKind::InvalidTupleIndex { index, length } => {
            assert_eq!(*index, 2);
            assert_eq!(*length, 2);
        }
        other => panic!("expected InvalidTupleIndex, got {other:?}"),
    }
}

#[test]
fn rejects_tuple_access_on_non_tuple_values() {
    let mut hir = load_hir("func main(): int { let value = 10; value.0 }");

    let err =
        TypeChecker::check(&mut hir).expect_err("non-tuples should reject tuple-style access");

    assert!(
        matches!(err.kind, TypeErrorKind::InvalidTupleAccessTarget { .. }),
        "expected InvalidTupleAccessTarget, got {:?}",
        err.kind
    );
}
