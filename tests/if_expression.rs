use std::path::PathBuf;
mod common;

#[test]
fn lowers_if_else_expression_used_as_variable_value() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/ifExpression.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();
    let stages = context.build_stages().unwrap();
    let ir = stages.ir_text();

    assert!(
        ir.contains("main"),
        "IR should contain main function:\n{ir}"
    );
    assert!(ir.contains("Cbr"), "IR should contain Cbr:\n{ir}");
}
