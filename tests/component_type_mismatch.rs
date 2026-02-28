use std::{path::PathBuf, sync::Arc};

use slynx::compiler::js::WebCompiler;

#[test]
fn test_component_type_mismatch_errors() {
    // SavioCodes | 2026-02-28 14:26 (America/Sao_Paulo)
    // Regression test: assigning B into prop typed as A must fail type checking.
    let context = slynx::SlynxContext::new(Arc::new(PathBuf::from(
        "slynx/component_type_mismatch.slynx",
    )))
    .unwrap();

    let result = context.start_compilation(WebCompiler::new());
    assert!(
        result.is_err(),
        "Expected type checker to reject incompatible component assignment",
    );
}
