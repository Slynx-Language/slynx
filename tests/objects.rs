use std::{path::PathBuf, sync::Arc};

#[test]
fn test_objects() {
    let context =
        slynx::SlynxContext::new(Arc::new(PathBuf::from("examples/objects.syx"))).unwrap();
    let output = context.compile().unwrap();

    assert_eq!(
        output
            .output_path()
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("sir")
    );
}
