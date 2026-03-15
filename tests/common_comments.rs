use std::{path::PathBuf, sync::Arc};

use slynx::compiler::js::WebCompiler;

#[test]
fn test_common_comments() {
    let context =
        slynx::SlynxContext::new(Arc::new(PathBuf::from("slynx/common_comments.slynx"))).unwrap();
    let output = context.compile(WebCompiler::new()).unwrap();

    assert_eq!(
        output
            .output_path()
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("js")
    );
    assert!(output.bytes().is_empty());
}
