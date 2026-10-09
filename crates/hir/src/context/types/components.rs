use std::fmt::Debug;

use common::{
    dedup_pooled,
    pool::{DedupPool, DedupPoolId},
};

use crate::{ComponentType, StructField, SymbolPointer, Visible, term::TermId};
use common::DedupPoolStorage;

dedup_pooled!(pub ComponentsPool {
    components: ComponentType,
});

impl ComponentsPool {
    pub fn insert_component(
        &self,
        name: SymbolPointer,
        properties: Vec<(SymbolPointer, TermId)>,
        children: Vec<DedupPoolId<ComponentType>>,
    ) -> DedupPoolId<ComponentType> {
        let s = ComponentType {
            name,
            properties: properties
                .into_iter()
                .map(|property| {
                    Visible::new(
                        common::VisibilityModifier::Public,
                        StructField {
                            ty: property.1,
                            name: property.0,
                        },
                    )
                })
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
