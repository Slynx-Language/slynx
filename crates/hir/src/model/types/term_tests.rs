use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

use common::pool::DedupPool;

use crate::{
    arrays::ArrayTerm,
    context::TypesContext,
    term::{ExtensionNode, PrimitiveType, Term, TermId, TermNode},
    vector::VectorTerm,
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
    let int_id = ctx.storage.insert_type(Term::signed_integer_type(32));
    let int_again = ctx.storage.insert_type(Term::signed_integer_type(32));

    assert_eq!(
        int_id, int_again,
        "same HirType must map to the same TermId (I1/I5)"
    );

    let signed32 =
        ctx.storage
            .terms
            .insert(Term::new_type(TermNode::Primitive(PrimitiveType::Signed {
                bitsize: 32,
            })));
    assert_eq!(
        int_id, signed32,
        "HirType::Int must collapse to Signed{{32}}"
    );
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
    let int_id = ctx.storage.insert_type(Term::signed_integer_type(32));

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

    let arr_term = ctx.storage.insert_type({
        let array = ctx.storage.insert_type(Term::extension_type(ArrayTerm));
        let length = ctx.storage.insert_type(Term::const_usize_type(4));
        Term::application(array, vec![int_id, length])
    });
    assert_eq!(
        children(arr_term),
        apply_children(arr_term),
        "Array children must mirror its Apply node (target + args)"
    );

    {
        let vec_term = {
            let vector = ctx.storage.insert_type(Term::extension_type(VectorTerm));
            let element = ctx.storage.insert_type(Term::signed_integer_type(32));
            ctx.storage
                .insert_type(Term::application(vector, vec![element]))
        };
        assert_eq!(
            children(vec_term),
            apply_children(vec_term),
            "Vector children must mirror its Apply node (target + args)"
        );
    }

    {
        let gc_term = ctx.storage.insert_type(Term::generic_component_type());
        assert!(
            children(gc_term).is_empty(),
            "GenericComponent has no children"
        );
    }
}
