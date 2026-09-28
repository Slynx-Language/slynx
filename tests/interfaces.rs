//! Integration tests for the interfaces feature (see
//! `docs/contributing/interfaces.md`).
//!
//! This is an integration test *by contract*: it asserts which programs the
//! language must be able to compile once interfaces land. The feature is not
//! implemented yet, so every corpus file carries a `// xfail:` marker and is
//! asserted to (still) be rejected today. When an example starts compiling,
//! the harness reports it so the marker can be upgraded to a plain positive
//! example.

mod common;

/// Compiles every `.slx`/`.syx` example in `examples/interfaces/` and honors
/// the `// xfail:` / `// xpass:` markers, exactly like the `generics` and
/// `enums` corpora.
#[test]
fn interface_examples() {
    common::run_examples("examples/interfaces", "interface");
}
