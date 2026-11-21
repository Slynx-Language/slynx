use std::fmt::Debug;

use common::{
    dedup_pooled,
    pool::{DedupPool, DedupPoolId},
};
use module_loader::FileId;

use crate::{ComponentChild, ComponentProperty, ComponentType, SymbolPointer, Visible};
use common::DedupPoolStorage;

dedup_pooled!(pub ComponentsPool {
    components: ComponentType,
});

impl ComponentsPool {
    pub fn insert_component(
        &self,
        name: SymbolPointer,
        owner: FileId,
        properties: Vec<ComponentProperty>,
        children: Vec<ComponentChild>,
    ) -> DedupPoolId<ComponentType> {
        let s = ComponentType {
            name,
            owner,
            properties: properties
                .into_iter()
                .map(|property| Visible::new(common::VisibilityModifier::Public, owner, property))
                .collect(),
            children,
        };
        self.components.insert(s)
    }
}

impl Debug for ComponentsPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentsPools")
            .field("components", &self.components)
            .finish()
    }
}
