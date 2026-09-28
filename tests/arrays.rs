mod common;

/// Compiles `examples/arrays.syx` end-to-end.
#[test]
fn test_arrays_and_slices() {
    common::compile_ok("examples/arrays.syx");
}
