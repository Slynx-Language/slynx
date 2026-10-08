mod common;

/// Discovers and compiles every `.slx` example in `examples/externs/`.
/// Each file is a focused test case for a specific extern declaration feature.
#[test]
fn extern_examples() {
    common::run_examples_ok("examples/externs", "extern");
}
