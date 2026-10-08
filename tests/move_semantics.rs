mod common;

/// Compiles every `.syx` example in `examples/move_semantics/` (references,
/// valid/invalid writes, move model) and honors the `// xfail:` / `// xpass:`
/// markers. `invalid_single_writer.syx` is marked `// xpass:` because the
/// single-writer model is not enforced yet (BM-210).
#[test]
fn move_semantics_examples() {
    common::run_examples("examples/move_semantics", "move semantics");
}
