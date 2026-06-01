mod common;

use slynx_hir::{
    ExpressionId,
    model::{HirExpression, HirExpressionKind},
};
use slynx_typechecker::{TypeChecker, error::TypeErrorKind};

#[test]
fn typechecker_rejects_function_call_with_extra_arg() {
    let mut hir = common::load_hir("examples/functionCall.syx");
    let args = common::find_main_call_args(&mut hir)
        .expect("expected to find a function call inside main");
    let template = args.first().expect("call should have at least one arg");
    args.push(HirExpression {
        id: ExpressionId::new(),
        ty: template.ty,
        kind: HirExpressionKind::Int(0),
        span: template.span,
    });

    let err = TypeChecker::check(hir)
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
