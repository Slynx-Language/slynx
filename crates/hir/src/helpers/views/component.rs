use common::pool::DedupPoolId;

use crate::{
    ComponentChild, ComponentProperty, ComponentPropertyKind, ComponentType, Visible,
    helpers::HirViewer,
};

impl HirViewer<'_, DedupPoolId<ComponentType>> {
    pub fn name(&self) -> &str {
        let name = self.hir.types[self.data].name;
        self.hir.get_name(name)
    }
    pub fn props(&self) -> &[Visible<ComponentProperty>] {
        &self.hir.types[self.data].properties
    }

    pub fn property_at(&self, idx: usize) -> Option<&Visible<ComponentProperty>> {
        self.props().get(idx)
    }

    /// Resolves the [`ComponentPropertyKind`] of the property at `index`, if any.
    pub fn property_kind(&self, index: usize) -> Option<ComponentPropertyKind> {
        self.props().get(index).map(|prop| prop.data.property_kind.clone())
    }

    pub fn children(&self) -> &[ComponentChild] {
        &self.hir.types[self.data].children
    }
}
