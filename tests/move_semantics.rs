use crate::common::STD_PATH;

mod common;
#[test]
fn test_references() {
    let ir = common::compile_ok("examples/move_semantics/references.syx");
    println!("References\n{ir}")
}

#[test]
fn test_invalid_writes() {
    let ir = slynx::compile_to_ir(
        "examples/move_semantics/invalid_writes.syx".into(),
        Some(STD_PATH.clone()),
    );
    let _ = ir.expect_err("Code should fail due to invalid writes");
}
#[test]
fn test_valid_writes() {
    let ir = common::compile_ok("examples/move_semantics/valid_writes.syx");
    println!("Valid writes\n{ir}")
}

#[test]
fn test_invalid_single_writer() {
    let ir = slynx::compile_to_ir(
        "examples/move_semantics/invalid_single_writer.syx".into(),
        Some(STD_PATH.clone()),
    );
    let err = ir.expect("Code shouldn't fail due to invalid single writer");
    println!("{err}");
}
