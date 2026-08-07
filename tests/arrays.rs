use std::path::PathBuf;
mod common;
#[test]
fn test_arrays_and_slices() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/arrays.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();
    let _ = context.compile().unwrap();
}
