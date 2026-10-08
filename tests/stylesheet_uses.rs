mod common;

/// Compiles every `.slx` example in `examples/styles/`, asserting each one
/// succeeds. Uses the shared all-compile folder harness from `common`.
#[test]
fn all_stylesheet_uses() {
    common::run_examples_ok("examples/styles", "stylesheet");
}
