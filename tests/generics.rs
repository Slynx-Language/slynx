mod common;

/// Discovers and compiles every `.slx` example in `examples/generics/`.
/// Each file is a focused test case for a generics feature. Files marked with
/// `// xfail:` are expected to be rejected (or to panic) today; any change in
/// their outcome is reported so the marker can be updated. Uses the shared
/// marker-driven harness from `common`.
#[test]
fn generic_examples() {
    common::run_examples("examples/generics", "generics");
}
