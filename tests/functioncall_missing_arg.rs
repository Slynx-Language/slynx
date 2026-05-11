mod common;

use slynx_typechecker::{TypeChecker, error::TypeErrorKind};

#[test]
fn typechecker_rejects_function_call_with_missing_arg() {
    let mut hir = common::load_hir("examples/functionCall.syx");
    let args = common::find_main_call_args(&mut hir)
        .expect("expected to find a function call inside main");
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
