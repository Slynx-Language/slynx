mod common;

/// Compiles `examples/booleans.syx` and asserts the generated artifact is a
/// `.sir` output.
#[test]
fn test_variables() {
    common::compile_ok_sir("examples/booleans.syx");
}
