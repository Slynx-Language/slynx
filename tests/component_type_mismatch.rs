mod common;

/// Regression test: assigning B into prop typed as A must fail type checking.
#[test]
fn test_component_type_mismatch_errors() {
    let err = common::compile_fails("examples/componentTypeMismatch.syx");
    assert!(
        !err.is_empty(),
        "Expected type checker to reject incompatible component assignment",
    );
}
