use std::path::PathBuf;

#[test]
fn test_objects() {
    let context = slynx::SlynxContext::new(PathBuf::from("examples/objects.syx"), None).unwrap();
    let output = context.compile().unwrap();

    assert_eq!(
        output
            .output_path()
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("sir")
    );
}
