use std::{path::PathBuf, sync::Arc};
#[test]
fn test_tuple() {
    let context = slynx::SlynxContext::new(Arc::new(PathBuf::from("slynx/tuplas.sly"))).unwrap();
    let output = context.compile().unwrap();
    assert_eq!(
        output
            .output_path()
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("sir")
    );
}

#[test]
fn test_tuple_access() {
    let context =
        slynx::SlynxContext::new(Arc::new(PathBuf::from("slynx/tuple_access.slynx"))).unwrap();
    let output = context.compile().unwrap();

    assert_eq!(
        output
            .output_path()
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("sir")
    );
}
