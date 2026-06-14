use std::path::PathBuf;
mod common;

#[test]
fn test_object_methods() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/objMethod.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();

    let result = context.compile().unwrap();
    println!("{}", result.ir().format_sir());
}
