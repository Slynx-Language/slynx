use std::path::PathBuf;
mod common;
#[test]
fn test_number_systems() {
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/numberSystems.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();
    let output = context.compile().unwrap();

    assert_eq!(
        output
            .output_path()
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("sir")
    );
}
