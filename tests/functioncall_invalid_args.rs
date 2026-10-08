mod common;

use slynx_hir::{HIRErrorKind, SlynxHir};

use crate::common::load_source;

/// Loads `source`, builds the HIR and asserts a call-arity error with the
/// expected argument counts.
fn rejects_call_arity(source: &str, expected: usize, received: usize) {
    let context = load_source(source);
    let modules = context.load_modules().expect("modules should load");
    let (_hir, err) = SlynxHir::new(&modules).expect_err("should reject wrong arg count");
    assert!(matches!(
        err.kind,
        HIRErrorKind::InvalidFuncallArgLength {
            expected_length: e,
            received_length: r,
            ..
        } if e == expected && r == received
    ));
}

#[test]
fn rejects_function_call_with_extra_arg() {
    rejects_call_arity(
        "func add(a: int, b: int): int { a + b } func main(): void { add(1, 2, 3) }",
        2,
        3,
    );
}

#[test]
fn rejects_function_call_with_missing_arg() {
    rejects_call_arity(
        "func add(a: int, b: int): int { a + b } func main(): void { add(1) }",
        2,
        1,
    );
}
