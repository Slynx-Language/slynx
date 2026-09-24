use std::hash::Hash;

use crate::{
    Result,
    term::{ExtensionNode, TermId, TermKind},
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ArrayTerm;

impl ExtensionNode for ArrayTerm {
    fn map_children(
        &self,
        _: &mut dyn FnMut(TermId) -> TermId,
    ) -> std::sync::Arc<dyn ExtensionNode> {
        std::sync::Arc::new(ArrayTerm)
    }

    fn try_map_children(
        &self,
        _: &mut dyn FnMut(TermId) -> Result<TermId>,
    ) -> Result<std::sync::Arc<dyn ExtensionNode>> {
        Ok(std::sync::Arc::new(ArrayTerm))
    }

    fn children(&self) -> Vec<TermId> {
        vec![]
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
