use std::path::PathBuf;

#[test]
fn test_tuple_access() {
    let context = slynx::SlynxContext::new(PathBuf::from("examples/tupleAccess.syx")).unwrap();
    let output = context.compile().unwrap();

    assert_eq!(
        output
            .output_path()
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("sir")
    );
}

/// Regression: tuple whose first field is a concrete Struct TypeId (not a Reference).
/// Previously panicked with IRTypeNotRecognized(TypeId(7)).
#[test]
fn test_tuple_object_and_string() {
    let context = slynx::SlynxContext::new(PathBuf::from("examples/tupleAccess.syx")).unwrap();
    let output = context.compile().unwrap();
    assert_eq!(
        output.output_path().extension().and_then(|e| e.to_str()),
        Some("sir")
    );
}

/// Two objects of the same type inside a tuple.
#[test]
fn test_tuple_two_objects() {
    let context = slynx::SlynxContext::new(PathBuf::from("examples/tupleTwoObjects.syx")).unwrap();
    let output = context.compile().unwrap();
    assert_eq!(
        output.output_path().extension().and_then(|e| e.to_str()),
        Some("sir")
    );
}

/// Nested tuple containing an object: ((Person, str), int).
#[test]
fn test_tuple_nested_object() {
    let context =
        slynx::SlynxContext::new(PathBuf::from("examples/tupleNestedObject.syx")).unwrap();
    let output = context.compile().unwrap();
    assert_eq!(
        output.output_path().extension().and_then(|e| e.to_str()),
        Some("sir")
    );
}
