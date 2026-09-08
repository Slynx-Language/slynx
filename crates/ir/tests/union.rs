use slynx_ir::{IRType, IRUnionFlags, SlynxIR};

#[test]
fn union_lifecycle_and_emission() {
    let mut ir = SlynxIR::new();
    let i32_ty = ir.int_type();
    let f64_ty = ir.float_type();

    // create_union_full (named union with variants + flags) — mirrors create_struct_full.
    let union_ty = ir.create_union_full("A", vec![i32_ty, f64_ty], IRUnionFlags::default());
    assert!(matches!(ir.get_type(union_ty), IRType::Union(_)));

    // size should be the largest variant (int=4, float32=4 -> 4).
    if let IRType::Union(id) = *ir.get_type(union_ty) {
        let union = ir.get_union_type(id);
        assert_eq!(union.get_variants().len(), 2);
        assert_eq!(union.size(), 4);
    }

    // create_union (empty by name) — mirrors create_struct.
    let b_ty = ir.create_union("B");
    assert!(matches!(ir.get_type(b_ty), IRType::Union(_)));

    // Textual emission.
    let out = ir.format_sir();
    assert!(out.contains("union %A{i32,f32};\n"), "got:\n{out}");
    assert!(out.contains("union %B{};\n"), "got:\n{out}");
}
