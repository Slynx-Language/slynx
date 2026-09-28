mod common;

/// Compiles `examples/commonComments.syx` and asserts the generated artifact
/// is a `.sir` output.
#[test]
fn test_common_comments() {
    common::compile_ok_sir("examples/commonComments.syx");
}
