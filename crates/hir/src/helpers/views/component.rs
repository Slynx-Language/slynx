use common::pool::DedupPoolId;

use crate::{ComponentType, HirType, SymbolPointer, helpers::HirViewer};

impl HirViewer<'_, DedupPoolId<ComponentType>> {
    pub fn name(&self) -> &str {
        let metadata = self.hir.types_module[self.data].metadata;
        let name = self.hir.types_module[metadata].name;
        self.hir.get_name(name)
    }
    pub fn props(&self) -> &[DedupPoolId<HirType>] {
        &self.hir.types_module[self.data].properties
    }
    pub fn prop_names(&self) -> &[SymbolPointer] {
        let metadata_id = self.hir.types_module[self.data].metadata;
        let props = &self.hir.types_module[metadata_id].properties;
        props
    }
}
