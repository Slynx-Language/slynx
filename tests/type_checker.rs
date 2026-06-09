mod common;

use slynx_hir::model::{HirDeclarationKind, HirStatementKind};
use slynx_typechecker::{TypeChecker, error::TypeErrorKind};

use crate::common::load_source;

// ─── Function call tests ───────────────────────────────────────────

#[test]
fn function_calls_work_with_mixed_declaration_order() {
    let hir = load_source("func bar(): void {} func main(): void { bar() }").unwrap();
    TypeChecker::check(hir).expect("function call should resolve with declaration ids");
}

#[test]
fn rejects_function_call_with_wrong_argument_type() {
    let hir =
        load_source("func takes_int(value: int): void {} func main(): void { takes_int(true) }")
            .unwrap();

    let err = TypeChecker::check(hir)
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
    let hir = load_source("func main(): int { let x = 12; }").unwrap();

    let err = TypeChecker::check(hir).expect_err("non-void functions must return a value");

    match &err.kind {
        TypeErrorKind::MissingReturnValue { expected } => {
            assert!(
                matches!(expected, slynx_hir::HirType::Int),
                "expected missing int return, got {expected:?}"
            );
        }
        other => panic!("expected MissingReturnValue, got {other:?}"),
    }
}

#[test]
fn preserves_non_expression_tail_statement_in_function_body() {
    let hir = load_source("func main(): void { let x = 12; }").unwrap();

    let reader = hir.files[0].read();
    let main_fn = &reader.declarations()[0].kind;

    let HirDeclarationKind::Function { statements, .. } = &main_fn else {
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
    let hir = load_source("func main(): void { while 10 { 0; } }").unwrap();

    let err = TypeChecker::check(hir).expect_err("while conditions should require bool");

    assert!(
        matches!(err.kind, TypeErrorKind::IncompatibleTypes { .. }),
        "expected IncompatibleTypes, got {:?}",
        err.kind
    );
}

#[test]
fn rejects_invalid_statement_inside_while_body() {
    let hir = load_source(
        "func takes_int(value: int): void {} func main(): void { while true { takes_int(false); } }",
    ).unwrap();

    let err = TypeChecker::check(hir).expect_err("while body statements should be type-checked");

    assert!(
        matches!(err.kind, TypeErrorKind::IncompatibleTypes { .. }),
        "expected IncompatibleTypes, got {:?}",
        err.kind
    );
}

// ─── Field / tuple access tests ────────────────────────────────────

#[test]
fn resolves_field_access_for_variables_typed_via_alias() {
    let hir = load_source(
        "object Person { age: int } alias PersonAlias = Person;
         func make_person(): PersonAlias { Person(age: 22) }
         func main(): int { let person = make_person(); person.age }",
    )
    .unwrap();
    TypeChecker::check(hir).expect("field access should resolve through aliases");
}

#[test]
fn resolves_tuple_access_for_tuple_variables() {
    let hir = load_source("func main(): int { let pair = (10, 20); pair.0 }").unwrap();
    TypeChecker::check(hir).expect("tuple access should resolve through the checker");
}

#[test]
fn resolves_named_field_access_after_tuple_access() {
    let hir = load_source(
        "object Person { age: int }
         func main(): int { let pair = (Person(age: 22), \"ok\"); pair.0.age }",
    )
    .unwrap();
    TypeChecker::check(hir).expect("named field access after tuple access should resolve cleanly");
}

#[test]
fn rejects_tuple_access_with_invalid_index() {
    let hir = load_source("func main(): int { let pair = (10, 20); pair.2 }").unwrap();

    let err = TypeChecker::check(hir).expect_err("tuple accesses should reject invalid indexes");

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
    let hir = load_source("func main(): int { let value = 10; value.0 }").unwrap();

    let err = TypeChecker::check(hir).expect_err("non-tuples should reject tuple-style access");

    assert!(
        matches!(err.kind, TypeErrorKind::InvalidTupleAccessTarget { .. }),
        "expected InvalidTupleAccessTarget, got {:?}",
        err.kind
    );
}
