use std::hash::Hash;

use common::pool::DedupPoolId;

use crate::{InterfaceType, term::ExtensionNode};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct InterfaceTerm {
    pub raw: DedupPoolId<InterfaceType>,
}

impl ExtensionNode for InterfaceTerm {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn dyn_eq(&self, other: &dyn ExtensionNode) -> bool {
        other.as_any().downcast_ref() == Some(self)
    }
    fn dyn_hash(&self, mut state: &mut dyn std::hash::Hasher) {
        self.hash(&mut state);
    }
    fn children(&self) -> Vec<super::term::TermId> {
        vec![]
    }
    fn kind(&self) -> super::term::TermKind {
        super::term::TermKind::Type
    }
    fn name(&self) -> &'static str {
        "Interface"
    }
    fn map_children(
        &self,
        _: &mut dyn FnMut(super::term::TermId) -> super::term::TermId,
    ) -> std::sync::Arc<dyn ExtensionNode> {
        std::sync::Arc::new(self.clone())
    }
    fn try_map_children(
        &self,
        _: &mut dyn FnMut(super::term::TermId) -> crate::Result<super::term::TermId>,
    ) -> crate::Result<std::sync::Arc<dyn ExtensionNode>> {
        Ok(std::sync::Arc::new(self.clone()))
    }
}
