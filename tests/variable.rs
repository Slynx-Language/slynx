mod common;

/// Compiles `examples/variables.syx` and asserts the generated artifact is a
/// `.sir` output.
#[test]
fn test_variables() {
    common::compile_ok_sir("examples/variables.syx");
}
