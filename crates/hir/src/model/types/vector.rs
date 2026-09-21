use std::hash::Hash;

use crate::term::{ExtensionNode, TermId, TermKind};

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct VectorTerm;

impl ExtensionNode for VectorTerm {
    fn children(&self) -> Vec<TermId> {
        vec![]
    }

    fn kind(&self) -> TermKind {
        TermKind::Type
    }

    fn name(&self) -> &'static str {
        "Vector"
    }

    fn dyn_eq(&self, other: &dyn ExtensionNode) -> bool {
        other.as_any().downcast_ref::<Self>().is_some()
    }

    fn dyn_hash(&self, mut state: &mut dyn std::hash::Hasher) {
        self.hash(&mut state);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
