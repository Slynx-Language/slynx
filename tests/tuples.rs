mod common;

/// Compiles `examples/tupleAccess.syx` (a regression fixture: the tuple's
/// first field is a concrete Struct TypeId, not a Reference), asserting the
/// generated artifact is a `.sir` output.
#[test]
fn test_tuple_access() {
    common::compile_ok_sir("examples/tupleAccess.syx");
}

/// Compiles `examples/tupleTwoObjects.syx`, asserting the generated artifact is
/// a `.sir` output.
#[test]
fn test_tuple_two_objects() {
    common::compile_ok_sir("examples/tupleTwoObjects.syx");
}

/// Compiles `examples/tupleNestedObject.syx`, asserting the generated artifact
/// is a `.sir` output.
#[test]
fn test_tuple_nested_object() {
    common::compile_ok_sir("examples/tupleNestedObject.syx");
}
