use crate::common::STD_PATH;

mod common;
#[test]
fn test_references() {
    let ir = common::compile_ok("examples/move_semantics/references.syx");
    println!("{ir}")
}

#[test]
fn test_invalid_writes() {
    let ir = slynx::compile_to_ir(
        "examples/move_semantics/invalid_writes.syx".into(),
        Some(STD_PATH.clone()),
    );
    println!("{ir:?}");
    let _ = ir.expect_err("Code should fail due to invalid writes");
}
