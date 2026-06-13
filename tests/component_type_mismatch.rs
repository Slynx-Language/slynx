use std::path::PathBuf;
mod common;
#[test]
fn test_component_type_mismatch_errors() {
    // SavioCodes | 2026-02-28 14:26 (America/Sao_Paulo)
    // Regression test: assigning B into prop typed as A must fail type checking.
    let context = slynx::SlynxContext::new(
        PathBuf::from("examples/componentTypeMismatch.syx"),
        Some(common::STD_PATH.clone()),
    )
    .unwrap();

    let result = context.compile();
    assert!(
        result.is_err(),
        "Expected type checker to reject incompatible component assignment",
    );
}
