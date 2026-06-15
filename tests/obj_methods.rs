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

#[test]
fn test_object_methods_with_multiple_methods() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/objMethods.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();

    let result = context.compile().unwrap();
    println!("{}", result.ir().format_sir());
}

#[test]
fn test_object_static_methods() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/objMethodStatic.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();

    let result = context.compile().unwrap();
    println!("{}", result.ir().format_sir());
}
