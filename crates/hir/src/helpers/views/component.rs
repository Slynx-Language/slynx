use common::pool::DedupPoolId;

use crate::{
    ComponentChild, ComponentProperty, ComponentType, StructField, Visible, helpers::HirViewer,
};

impl HirViewer<'_, DedupPoolId<ComponentType>> {
    pub fn name(&self) -> &str {
        let name = self.hir.types[self.data].name;
        self.hir.get_name(name)
    }
    pub fn props(&self) -> &[Visible<ComponentProperty>] {
        &self.hir.types[self.data].properties
    }

    pub fn children(&self) -> &[ComponentChild] {
        &self.hir.types[self.data].children
    }
}
