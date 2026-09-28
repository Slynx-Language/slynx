mod common;

/// Compiles `examples/ifExpression.syx` and inspects the generated IR.
#[test]
fn lowers_if_else_expression_used_as_variable_value() {
    let stages = common::load_example("examples/ifExpression.syx")
        .build_stages()
        .unwrap();
    let ir = stages.ir_text();

    assert!(
        ir.contains("main"),
        "IR should contain main function:\n{ir}"
    );
    assert!(ir.contains("Cbr"), "IR should contain Cbr:\n{ir}");
}
