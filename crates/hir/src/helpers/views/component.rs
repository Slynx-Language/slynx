use common::pool::DedupPoolId;

use crate::{ComponentType, HirType, SymbolPointer, helpers::HirViewer};

impl HirViewer<'_, DedupPoolId<ComponentType>> {
    pub fn name(&self) -> &str {
        let metadata = self.hir.types[self.data].metadata;
        let name = self.hir.types[metadata].name;
        self.hir.get_name(name)
    }
    pub fn props(&self) -> &[DedupPoolId<HirType>] {
        &self.hir.types[self.data].properties
    }
    pub fn prop_names(&self) -> &[SymbolPointer] {
        let metadata_id = self.hir.types[self.data].metadata;

        (&self.hir.types[metadata_id].properties) as _
    }
    pub fn children(&self) -> &[DedupPoolId<ComponentType>] {
        &self.hir.types[self.data].children
    }
}
