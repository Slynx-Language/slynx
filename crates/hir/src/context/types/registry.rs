use common::pool::DedupPoolId;
use dashmap::DashMap;

use crate::{HirType, SymbolPointer};

#[derive(Debug)]
/// Registers names in the type namespace and resolves them back to type ids.
///
/// This owns the name→type-id mapping but knows nothing about how types are
/// stored ([`super::storage::TypeStorage`]) or resolved to shapes.
pub struct TypeRegistry {
    names: DashMap<SymbolPointer, DedupPoolId<HirType>>,
}
impl Default for TypeRegistry {
    fn default() -> Self {
        Self {
            names: DashMap::new(),
        }
    }
}
impl TypeRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers (or overwrites) `name` so it resolves to `ty`.
    pub fn register(&self, name: SymbolPointer, ty: DedupPoolId<HirType>) {
        self.names.insert(name, ty);
    }

    ///Retrieves the DedupPoolId<HirType> of the provided `name` on the currentContext
    pub fn get_id_of_name(&self, name: &SymbolPointer) -> Option<DedupPoolId<HirType>> {
        self.names.get(name).map(|v| *v.value())
    }
}