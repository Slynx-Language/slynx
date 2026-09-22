use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

use common::pool::DedupPool;
use module_loader::ASTBuiltin;

use crate::{
    HirType,
    arrays::ArrayTerm,
    context::TypesContext,
    term::{ExtensionNode, PrimitiveType, Term, TermId, TermNode},
};

fn hash_of(term: Term) -> u64 {
    let mut hasher = DefaultHasher::new();
    term.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn dedup_equivalent_primitives() {
    let pool = DedupPool::new();
    let unsigned = Term::new_type(TermNode::Primitive(PrimitiveType::Unsigned { bitsize: 32 }));
    let signed = Term::new_type(TermNode::Primitive(PrimitiveType::Signed { bitsize: 32 }));

    let first = pool.insert(unsigned.clone());
    let second = pool.insert(unsigned);

    assert_eq!(first, second, "equal primitives must dedup to the same id");
    assert_ne!(
        pool.insert(signed),
        first,
        "Signed{{32}} must not alias Unsigned{{32}}"
    );
}

#[test]
fn boolean_is_unsigned1() {
    assert_eq!(
        PrimitiveType::boolean_type(),
        PrimitiveType::Unsigned { bitsize: 1 }
    );
}

#[test]
fn collapse_parity() {
    let ctx = TypesContext::new();
    let int_id = ctx.storage.insert_type(HirType::Int);
    let float_id = ctx.storage.insert_type(HirType::Float);
    let bool_id = ctx.storage.insert_type(HirType::Bool);

    let int_term = ctx.to_term(int_id);
    let int_term_again = ctx.to_term(int_id);
    assert_eq!(
        int_term, int_term_again,
        "same HirType must map to the same TermId (I1/I5)"
    );

    let signed32 =
        ctx.storage
            .terms
            .insert(Term::new_type(TermNode::Primitive(PrimitiveType::Signed {
                bitsize: 32,
            })));
    assert_eq!(
        int_term, signed32,
        "HirType::Int must collapse to Signed{{32}}"
    );

    let float_term = ctx.to_term(float_id);
    let float32 = ctx
        .storage
        .terms
        .insert(Term::new_type(TermNode::Primitive(PrimitiveType::Float32)));
    assert_eq!(
        float_term, float32,
        "HirType::Float must collapse to Float32"
    );

    let bool_term = ctx.to_term(bool_id);
    let unsigned1 = ctx.storage.terms.insert(Term::new_type(TermNode::Primitive(
        PrimitiveType::boolean_type(),
    )));
    assert_eq!(
        bool_term, unsigned1,
        "HirType::Bool must collapse to Unsigned{{1}}"
    );

    assert_ne!(int_term, float_term);
    assert_ne!(int_term, bool_term);
    assert_ne!(float_term, bool_term);
}

#[test]
fn collapse_ast_builtin_boundary() {
    let ctx = TypesContext::new();
    let to_id = |b: ASTBuiltin| ctx.storage.insert_type(HirType::from(b));
    let to_term = |b: ASTBuiltin| ctx.to_term(to_id(b));

    for builtin in [
        ASTBuiltin::Int(8),
        ASTBuiltin::Int(32),
        ASTBuiltin::Int(64),
        ASTBuiltin::Uint(16),
        ASTBuiltin::Uint(32),
    ] {
        assert_eq!(
            to_term(builtin),
            to_term(ASTBuiltin::Int(32)),
            "int/uint of any bitsize must collapse to the same TermId (I5)"
        );
    }

    for builtin in [ASTBuiltin::F16, ASTBuiltin::F32, ASTBuiltin::F64] {
        assert_eq!(
            to_term(builtin),
            to_term(ASTBuiltin::F32),
            "f16/f32/f64 must collapse to the same TermId (I5)"
        );
    }

    let bool_term = to_term(ASTBuiltin::Boolean);
    assert_eq!(
        bool_term,
        ctx.to_term(ctx.storage.insert_type(HirType::Bool))
    );
    assert_ne!(bool_term, to_term(ASTBuiltin::Int(32)));
    assert_ne!(bool_term, to_term(ASTBuiltin::F32));
}

#[test]
fn extension_hash_deterministic() {
    let ext_term = || {
        Term::new_type(TermNode::Extension(
            Arc::new(ArrayTerm) as Arc<dyn ExtensionNode>
        ))
    };

    let first = ext_term();
    let second = ext_term();
    assert_eq!(
        first, second,
        "structurally-equal extension nodes must be equal (I1)"
    );
    assert_eq!(
        hash_of(first),
        hash_of(second),
        "hash of structurally-equal extension nodes must be deterministic (I2)"
    );

    let pool = DedupPool::new();
    let one = pool.insert(ext_term());
    let two = pool.insert(ext_term());
    assert_eq!(one, two, "extension terms must dedup like any other term");
}

#[test]
fn children_shape() {
    let ctx = TypesContext::new();
    let int_id = ctx.storage.insert_type(HirType::Int);

    let children = |term_id: TermId| ctx.storage.terms[term_id].children();
    let apply_children = |term_id: TermId| {
        let node = &ctx.storage.terms[term_id].node();
        let TermNode::Apply { target, args } = node else {
            panic!("expected Apply node, got {node:?}");
        };
        let mut out = Vec::with_capacity(args.len() + 1);
        out.push(*target);
        out.extend(args.iter().copied());
        out
    };

    let arr_term = ctx.to_term(ctx.storage.insert_type(HirType::Array(int_id, 4)));
    assert_eq!(
        children(arr_term),
        apply_children(arr_term),
        "Array children must mirror its Apply node (target + args)"
    );

    let vec_term = ctx.to_term(ctx.storage.insert_type(HirType::Vector(int_id)));
    assert_eq!(
        children(vec_term),
        apply_children(vec_term),
        "Vector children must mirror its Apply node (target + args)"
    );

    let gc_term = ctx.to_term(ctx.storage.insert_type(HirType::GenericComponent));
    assert!(
        children(gc_term).is_empty(),
        "GenericComponent has no children"
    );
}
