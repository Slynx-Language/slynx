use std::path::PathBuf;
mod common;

#[test]
fn test_externs() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/externs.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();
    context.compile().unwrap();
}
