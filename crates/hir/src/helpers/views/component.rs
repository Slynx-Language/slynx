use common::{FrontendSymbol, SymbolPointer, pool::DedupPoolId};
use module_loader::FileId;

use crate::{
    ComponentChild, ComponentProperty, ComponentPropertyKind, ComponentType, Visible,
    helpers::HirViewer,
};

impl HirViewer<'_, DedupPoolId<ComponentType>> {
    pub fn owner(&self) -> FileId {
        self.hir.types.get_component_owner(self.data)
    }

    pub fn name(&self) -> SymbolPointer<FrontendSymbol> {
        self.hir.types[self.data].name
    }
    pub fn props(&self) -> &[Visible<ComponentProperty>] {
        &self.hir.types[self.data].properties
    }

    pub fn property_at(&self, idx: usize) -> Option<&Visible<ComponentProperty>> {
        self.props().get(idx)
    }

    /// Resolves the [`ComponentPropertyKind`] of the property at `index`, if any.
    pub fn property_kind(&self, index: usize) -> Option<ComponentPropertyKind> {
        self.props()
            .get(index)
            .map(|prop| prop.data.property_kind.clone())
    }

    pub fn children(&self) -> &[ComponentChild] {
        &self.hir.types[self.data].children
    }
}
