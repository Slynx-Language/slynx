use common::pool::DedupPoolId;

use crate::{ComponentType, StructField, Visible, helpers::HirViewer};

impl HirViewer<'_, DedupPoolId<ComponentType>> {
    pub fn name(&self) -> &str {
        let name = self.hir.types[self.data].name;
        self.hir.get_name(name)
    }
    pub fn props(&self) -> &[Visible<StructField>] {
        &self.hir.types[self.data].properties
    }

    pub fn children(&self) -> &[DedupPoolId<ComponentType>] {
        &self.hir.types[self.data].children
    }
}
