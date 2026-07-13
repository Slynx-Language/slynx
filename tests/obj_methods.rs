use std::path::PathBuf;
mod common;

#[test]
fn test_object_methods() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/objMethod.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();

    let _modules = context
        .load_modules()
        .expect("modules should load (object-method resolution not yet implemented)");
}

#[test]
fn test_object_methods_with_multiple_methods() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/objMethods.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();

    let _modules = context
        .load_modules()
        .expect("modules should load (object-method resolution not yet implemented)");
}

#[test]
fn test_object_static_methods() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/objMethodStatic.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();

    let _modules = context
        .load_modules()
        .expect("modules should load (static-method resolution not yet implemented)");
}
