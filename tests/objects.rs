use std::path::PathBuf;
mod common;

#[test]
fn test_objects() {
    let mut context = slynx::SlynxContext::new(
        PathBuf::from("examples/objects.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();

    // The HIR builder handles objects but codegen does not yet
    // recognize object types.  Verify that modules load.
    let _modules = context.load_modules().expect("modules should load");
}
