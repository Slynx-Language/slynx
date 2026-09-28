mod common;

/// The HIR builder handles objects but codegen does not yet recognize object
/// types.  Verify that modules load.
#[test]
fn test_objects() {
    let _modules = common::load_example("examples/objects.syx")
        .load_modules()
        .expect("modules should load");
}
