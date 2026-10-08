mod common;

/// Compiles `examples/while.syx` and asserts the generated artifact is a `.sir`
/// output.
#[test]
fn test_while() {
    common::compile_ok_sir("examples/while.syx");
}
