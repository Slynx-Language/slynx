mod common;
#[test]
fn test_references() {
    let ir = common::compile_ok("examples/move_semantics/references.syx");
    println!("{ir}")
}
