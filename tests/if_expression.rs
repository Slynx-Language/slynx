use std::{path::PathBuf, sync::Arc};

#[test]
fn lowers_if_else_expression_used_as_variable_value() {
    let context =
        slynx::SlynxContext::new(Arc::new(PathBuf::from("examples/ifExpression.syx"))).unwrap();
    let stages = context.build_stages().unwrap();
    let ir = stages.ir_text();

    assert!(ir.contains("Cbr"));
    assert!(ir.contains("Br"));
    assert!(ir.contains("LabelArg"));
    assert!(ir.contains("I32"));
}
