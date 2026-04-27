use frontend::hir::{SlynxHir, model::HirType};

#[test]
fn test_create_tuple_type() {
    let mut hir = SlynxHir::new();
    let int_id = hir.int32_type();
    let float_id = hir.float32_type();

    let tuple_id = hir.add_unnamed_type(HirType::Tuple {
        fields: vec![int_id, float_id],
    });

    match hir.get_type(&tuple_id) {
        HirType::Tuple { fields } => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0], int_id);
            assert_eq!(fields[1], float_id);
        }
        other => panic!("expected HirType::Tuple, got {other:?}"),
    }
}

#[test]
fn test_create_empty_tuple_type() {
    let mut hir = SlynxHir::new();

    let tuple_id = hir.add_unnamed_type(HirType::Tuple { fields: vec![] });

    match hir.get_type(&tuple_id) {
        HirType::Tuple { fields } => {
            assert_eq!(fields.len(), 0);
        }
        other => panic!("expected HirType::Tuple, got {other:?}"),
    }
}

#[test]
fn test_tuple_fields_are_independent_type_ids() {
    let mut hir = SlynxHir::new();
    let int_id = hir.int32_type();
    let str_id = hir.str_type();
    let bool_id = hir.bool_type();

    let tuple_id = hir.add_unnamed_type(HirType::Tuple {
        fields: vec![int_id, str_id, bool_id],
    });

    match hir.get_type(&tuple_id) {
        HirType::Tuple { fields } => {
            assert_eq!(fields.len(), 3);
            assert_eq!(fields[0], int_id);
            assert_eq!(fields[1], str_id);
            assert_eq!(fields[2], bool_id);
            assert_ne!(fields[0], fields[1]);
            assert_ne!(fields[1], fields[2]);
            assert_ne!(fields[0], fields[2]);
        }
        other => panic!("expected HirType::Tuple, got {other:?}"),
    }
}

#[test]
fn test_two_tuple_types_are_independent() {
    let mut hir = SlynxHir::new();
    let int_id = hir.int32_type();
    let float_id = hir.float32_type();

    let tuple_a = hir.add_unnamed_type(HirType::Tuple {
        fields: vec![int_id],
    });

    let tuple_b = hir.add_unnamed_type(HirType::Tuple {
        fields: vec![float_id],
    });

    assert_ne!(
        tuple_a, tuple_b,
        "two separately inserted tuples should have distinct IDs"
    );
}
