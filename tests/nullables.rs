use std::path::PathBuf;
mod common;
#[test]
fn test_nullable_types() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/nullables.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();
    let _ = context.compile().unwrap();
}
