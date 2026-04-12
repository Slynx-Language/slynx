use std::{path::PathBuf, sync::Arc};

#[test]
fn test_trailing_dot_float() {
    let context =
        slynx::SlynxContext::new(Arc::new(PathBuf::from("slynx/trailing_float.slynx"))).unwrap();
    let output = context.compile().unwrap();

    assert_eq!(
        output
            .output_path()
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("sir")
    );
}
