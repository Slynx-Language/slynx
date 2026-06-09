mod common;

use slynx_hir::HIRErrorKind;

use crate::common::load_source;

#[test]
fn rejects_function_call_with_extra_arg() {
    let err =
        load_source("func add(a: int, b: int): int { a + b } func main(): void { add(1, 2, 3) }")
            .expect_err("Hir should fail its generation due to extra arg");

    match &err.kind {
        HIRErrorKind::InvalidFuncallArgLength {
            expected_length,
            received_length,
            ..
        } => {
            assert_eq!(*expected_length, 2);
            assert_eq!(*received_length, 3);
        }
        other => panic!("expected InvalidFuncallArgLength, got {other:?}"),
    }
}

#[test]
fn rejects_function_call_with_missing_arg() {
    let err = load_source("func add(a: int, b: int): int { a + b } func main(): void { add(1) }")
        .expect_err("Hir should fail its generation due to missing args");

    match &err.kind {
        HIRErrorKind::InvalidFuncallArgLength {
            expected_length,
            received_length,
            ..
        } => {
            assert_eq!(*expected_length, 2);
            assert_eq!(*received_length, 1);
        }
        other => panic!("expected InvalidFuncallArgLength, got {other:?}"),
    }
}
