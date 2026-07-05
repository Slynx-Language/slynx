mod common;

use slynx_hir::{HIRErrorKind, SlynxHir};

use crate::common::load_source;

#[test]
fn rejects_function_call_with_extra_arg() {
    let mut context =
        load_source("func add(a: int, b: int): int { a + b } func main(): void { add(1, 2, 3) }");
    let modules = context
        .load_modules()
        .expect("modules should load");
    let (_hir, err) = SlynxHir::new(&modules).expect_err("should reject extra arg");
    assert!(matches!(
        err.kind,
        HIRErrorKind::InvalidFuncallArgLength {
            expected_length: 2,
            received_length: 3,
            ..
        }
    ));
}

#[test]
fn rejects_function_call_with_missing_arg() {
    let mut context =
        load_source("func add(a: int, b: int): int { a + b } func main(): void { add(1) }");
    let modules = context
        .load_modules()
        .expect("modules should load");
    let (_hir, err) = SlynxHir::new(&modules).expect_err("should reject missing arg");
    assert!(matches!(
        err.kind,
        HIRErrorKind::InvalidFuncallArgLength {
            expected_length: 2,
            received_length: 1,
            ..
        }
    ));
}
