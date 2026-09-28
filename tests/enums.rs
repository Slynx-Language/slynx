mod common;

/// Discovers and compiles every `.slx` example in `examples/enums/`.
/// Each file is a focused test case for a single enum language feature.
/// Files marked with `// xfail:` are expected to be rejected (or to panic)
/// today; any change in their outcome is reported so the marker can be updated.
#[test]
fn enum_examples() {
    common::run_examples("examples/enums", "enum");
}
