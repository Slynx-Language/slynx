use std::hash::Hash;

use crate::term::{ExtensionNode, TermId, TermKind};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ArrayTerm {
    ty: TermId,
    length: usize,
}
impl ArrayTerm {
    pub fn new(ty: TermId, length: usize) -> Self {
        Self { ty, length }
    }
}

impl ExtensionNode for ArrayTerm {
    fn children(&self) -> Vec<TermId> {
        vec![self.ty]
    }

    fn kind(&self) -> TermKind {
        TermKind::Type
    }

    fn name(&self) -> &'static str {
        "Array"
    }

    fn dyn_eq(&self, other: &dyn ExtensionNode) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }

    fn dyn_hash(&self, mut state: &mut dyn std::hash::Hasher) {
        self.hash(&mut state);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
