use std::ops::Index;

use common::pool::{Pool, PoolId};

use crate::{ComponentType, HirType, StructType, SymbolPointer};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ComponentDefinition {
    name: SymbolPointer,
    properties: Vec<SymbolPointer>,
}

pub struct ComponentsPool {
    pool: Pool<ComponentType>,
    bodies: Pool<ComponentDefinition>,
}

impl ComponentsPool {
    pub fn new() -> Self {
        Self {
            pool: Pool::new(),
            bodies: Pool::new(),
        }
    }
    pub fn insert(
        &self,
        name: SymbolPointer,
        properties: Vec<(SymbolPointer, PoolId<HirType>)>,
        children: Vec<PoolId<ComponentType>>,
    ) -> PoolId<ComponentType> {
        let (names, properties) = properties.into_iter().unzip();
        let s = ComponentType {
            properties: properties,
            children,
        };
        self.bodies.insert(ComponentDefinition {
            name,
            properties: names,
        });
        self.pool.insert(s)
    }
    pub fn deffinition_of(&self, ty: PoolId<ComponentType>) -> &ComponentDefinition {
        self.bodies.get(unsafe { PoolId::from_raw(ty.inner()) })
    }
}

impl Index<PoolId<ComponentType>> for ComponentsPool {
    type Output = ComponentType;
    fn index(&self, index: PoolId<ComponentType>) -> &Self::Output {
        self.pool.get(index)
    }
}
