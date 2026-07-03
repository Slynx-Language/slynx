use std::fmt::Debug;

use common::{
    dedup_pooled,
    pool::{DedupPool, DedupPoolId},
};

use crate::{ComponentType, HirType, SymbolPointer};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ComponentDefinition {
    pub(crate) name: SymbolPointer,
    pub(crate) properties: Vec<SymbolPointer>,
}

dedup_pooled!(pub ComponentsPool {
    components: ComponentType,
    bodies: ComponentDefinition
});

impl ComponentsPool {
    pub fn insert(
        &self,
        name: SymbolPointer,
        properties: Vec<(SymbolPointer, DedupPoolId<HirType>)>,
        children: Vec<DedupPoolId<ComponentType>>,
    ) -> (DedupPoolId<ComponentType>, DedupPoolId<ComponentDefinition>) {
        let (names, properties) = properties.into_iter().unzip();
        let def = self.bodies.insert(ComponentDefinition {
            name,
            properties: names,
        });
        let s = ComponentType {
            properties,
            children,
            metadata: def,
        };
        let comp = self.components.insert(s);
        (comp, def)
    }
}

impl Debug for ComponentsPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentsPools")
            .field("components", &self.components)
            .field("bodies", &self.bodies)
            .finish()
    }
}
