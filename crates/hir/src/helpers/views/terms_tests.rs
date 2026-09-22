#[cfg(test)]
mod views_terms_tests {
    use common::{FrontendSymbol, SymbolsModule, VisibilityModifier};

    use crate::{
        EnumVariantType, HirType, SlynxHir, SymbolPointer, arrays::ArrayTerm,
        context::TypesContext, generic_component::GenericComponentTerm, helpers::Visible,
        store::HirStore, vector::VectorTerm,
    };

    fn hir_ctx<'a>(symbols: &'a SymbolsModule<FrontendSymbol>) -> SlynxHir<'a> {
        SlynxHir {
            symbols_resolver: symbols,
            symbols_registry: Default::default(),
            types: TypesContext::new(),
            store: HirStore::new(),
        }
    }

    #[test]
    fn name_matches_hirtypes_viewer_over_representative_types() {
        let symbols = SymbolsModule::<FrontendSymbol>::new();
        let hir = hir_ctx(&symbols);

        let int_id = hir.types.create_type(HirType::Int);
        let float_id = hir.types.create_type(HirType::Float);
        let bool_id = hir.types.create_type(HirType::Bool);
        let str_id = hir.types.create_type(HirType::Str);
        let void_id = hir.types.create_type(HirType::Void);
        let generic_id = hir.types.create_type(HirType::GenericComponent);

        let person_name: SymbolPointer = symbols.intern("Person");
        let field_name: SymbolPointer = symbols.intern("age");
        let person_id = hir.types.create_struct_type(
            person_name,
            vec![Visible::new(
                VisibilityModifier::Public,
                (field_name, int_id),
            )],
            Vec::new(),
        );

        let func_id = hir
            .types
            .create_function_type(vec![int_id, bool_id], float_id);
        let tuple_id = hir.types.create_tuple_type(vec![int_id, bool_id]);
        let immut_id = hir.types.create_type(HirType::ImutableRef(int_id));
        let mut_id = hir.types.create_type(HirType::MutableRef(int_id));
        let person_ref = hir
            .types
            .create_type(HirType::new_generic_ref(person_id, vec![int_id]));
        let array_id = hir.types.create_type(HirType::Array(int_id, 4));
        let vector_id = hir.types.create_type(HirType::Vector(int_id));

        let ids = [
            int_id, float_id, bool_id, str_id, void_id, generic_id, person_id, func_id, tuple_id,
            immut_id, mut_id, person_ref, array_id, vector_id,
        ];

        for id in ids {
            let term = hir.types.to_term(id);
            let hir_name = hir.view(id).name();
            let term_name = hir.view(term).name();
            assert_eq!(
                hir_name, term_name,
                "name parity (I11) broken for type {id:?} ({term:?})"
            );
        }
    }

    #[test]
    fn name_renders_expected_texts() {
        let symbols = SymbolsModule::<FrontendSymbol>::new();
        let hir = hir_ctx(&symbols);

        let int_id = hir.types.create_type(HirType::Int);
        let bool_id = hir.types.create_type(HirType::Bool);
        let float_id = hir.types.create_type(HirType::Float);
        let str_id = hir.types.create_type(HirType::Str);
        let void_id = hir.types.create_type(HirType::Void);
        let generic_id = hir.types.create_type(HirType::GenericComponent);
        let array_id = hir.types.create_type(HirType::Array(int_id, 4));
        let vector_id = hir.types.create_type(HirType::Vector(int_id));
        let tuple_id = hir.types.create_tuple_type(vec![int_id, bool_id]);
        let func_id = hir
            .types
            .create_function_type(vec![int_id, bool_id], float_id);

        let cases = [
            (int_id, "int"),
            (bool_id, "bool"),
            (float_id, "float32"),
            (str_id, "str"),
            (void_id, "void"),
            (generic_id, "anycomponent"),
            (array_id, "[4]int"),
            (vector_id, "[]int"),
            (tuple_id, "(int,bool)"),
            (func_id, "func(int,bool)->float32"),
        ];

        for (id, expected) in cases {
            let term = hir.types.to_term(id);
            assert_eq!(hir.view(term).name(), expected, "term name for {expected}");
        }
    }

    #[test]
    fn children_shape_via_viewer() {
        let symbols = SymbolsModule::<FrontendSymbol>::new();
        let hir = hir_ctx(&symbols);

        let int_id = hir.types.create_type(HirType::Int);
        let bool_id = hir.types.create_type(HirType::Bool);

        let int_term = hir.types.to_term(int_id);
        let bool_term = hir.types.to_term(bool_id);
        let float_id = hir.types.create_type(HirType::Float);
        let float_term = hir.types.to_term(float_id);

        // Array: Apply(target = Array ext, args = [elem, len constant]).
        let array = hir.view(
            hir.types
                .to_term(hir.types.create_type(HirType::Array(int_id, 4))),
        );
        let array_children = { array.children() };
        assert_eq!(array_children.len(), 3);
        assert!(
            hir.view(array_children[0])
                .is_extension()
                .map(|ext| ext.dyn_eq(&ArrayTerm))
                .is_some()
        );
        assert_eq!(array.is_array(), Some((int_term, 4)));
        assert_eq!(array.is_vector(), None);

        // Vector: Apply(target = Vector ext, args = [elem]).
        let vector = hir.view(
            hir.types
                .to_term(hir.types.create_type(HirType::Vector(int_id))),
        );
        let vector_children = vector.children();
        assert!(
            hir.view(vector_children[0])
                .is_extension()
                .map(|ext| ext.dyn_eq(&VectorTerm))
                .is_some()
        );
        assert_eq!(vector.children(), vec![vector_children[0], int_term]);
        assert_eq!(vector.is_vector(), Some(int_term));
        assert_eq!(vector.is_array(), None);

        // Func: children = args + ret.
        let func = hir.view(
            hir.types.to_term(
                hir.types
                    .create_function_type(vec![int_id, bool_id], float_id),
            ),
        );
        assert_eq!(
            func.is_function(),
            Some((vec![int_term, bool_term], float_term))
        );
        assert_eq!(func.children(), vec![int_term, bool_term, float_term]);

        // Tuple: children = fields.
        let tuple = hir.view(
            hir.types
                .to_term(hir.types.create_tuple_type(vec![int_id, bool_id])),
        );
        assert_eq!(tuple.children(), vec![int_term, bool_term]);

        // Ref: children = target.
        let immut = hir.view(
            hir.types
                .to_term(hir.types.create_type(HirType::ImutableRef(int_id))),
        );
        assert_eq!(immut.children(), vec![int_term]);
        assert_eq!(immut.is_imutable_ref(), Some(int_term));
        assert!(immut.is_ref());

        // GenericComponent extension has no children and is not a ref.
        let gc = hir.view(
            hir.types
                .to_term(hir.types.create_type(HirType::GenericComponent)),
        );
        assert_eq!(
            gc.is_extension()
                .map(|ext| ext.dyn_eq(&GenericComponentTerm))
                .is_some(),
            true
        );
        assert!(gc.children().is_empty());
    }

    #[test]
    fn descriptor_predicates() {
        let symbols = SymbolsModule::<FrontendSymbol>::new();
        let hir = hir_ctx(&symbols);

        let int_id = hir.types.create_type(HirType::Int);

        let person_name: SymbolPointer = symbols.intern("Person");
        let field_name: SymbolPointer = symbols.intern("age");
        let person_id = hir.types.create_struct_type(
            person_name,
            vec![Visible::new(
                VisibilityModifier::Public,
                (field_name, int_id),
            )],
            Vec::new(),
        );

        let color_name: SymbolPointer = symbols.intern("Color");
        let enum_id = hir.types.create_enum_type(
            color_name,
            vec![EnumVariantType {
                name: symbols.intern("Red"),
                payload: vec![int_id],
                discriminant: 0,
            }],
        );

        let label_name: SymbolPointer = symbols.intern("Label");
        let component_id = hir.types.create_component_type(label_name, vec![], vec![]);

        let person_term = hir.view(hir.types.to_term(person_id));
        assert!(person_term.is_struct().is_some());
        assert!(person_term.is_enum().is_none());

        let enum_term = hir.view(hir.types.to_term(enum_id));
        assert!(enum_term.is_enum().is_some());
        assert!(enum_term.is_component().is_none());

        let component_term = hir.view(hir.types.to_term(component_id));
        assert!(component_term.is_component().is_some());

        let generic_term = hir.view(hir.types.to_term(hir.types.create_type(
            HirType::GenericParam {
                index: 0,
                name: symbols.intern("T"),
            },
        )));
        assert!(generic_term.is_generic().is_some());
        assert_eq!(generic_term.name(), "T");
    }

    #[test]
    fn mutable_ref_predicate() {
        let symbols = SymbolsModule::<FrontendSymbol>::new();
        let hir = hir_ctx(&symbols);

        let int_id = hir.types.create_type(HirType::Int);
        let int_term = hir.types.to_term(int_id);

        let mutref = hir.view(
            hir.types
                .to_term(hir.types.create_type(HirType::MutableRef(int_id))),
        );
        assert_eq!(mutref.is_mutable_ref(), Some(int_term));
        assert!(mutref.is_ref());
    }
}
