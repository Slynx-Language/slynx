mod common;

/// Compiles `examples/numberSystems.syx` and asserts the generated artifact is
/// a `.sir` output.
#[test]
fn test_number_systems() {
    common::compile_ok_sir("examples/numberSystems.syx");
}
