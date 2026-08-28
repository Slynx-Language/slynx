use common::{VisibilityModifier, pool::DedupPoolId};

use crate::{
    DeclarationId, HirFunctionDeclaration, HirType, StructType, SymbolPointer, TupleType,
    helpers::{HirViewer, Visible},
};

impl HirViewer<'_, DedupPoolId<StructType>> {
    pub fn name(&self) -> SymbolPointer {
        self.hir.types_module.get_struct_name(self.data)
    }

    pub fn fields(&self) -> &[Visible<SymbolPointer>] {
        self.hir.types_module.get_struct_fields(self.data)
    }

    pub fn field_types(&self) -> &[DedupPoolId<HirType>] {
        self.hir.types_module.get_struct_field_types(self.data)
    }

    pub fn signature(&self) -> Vec<(&Visible<SymbolPointer>, &DedupPoolId<HirType>)> {
        self.hir.types_module.get_struct_signature(self.data)
    }
    pub fn methods(&self) -> &[Visible<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)>] {
        let metadata = self.hir.types_module[self.data].metadata;
        &self.hir.types_module[metadata].methods
    }
    pub fn public_methods(
        &self,
    ) -> impl Iterator<Item = &Visible<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)>>
    {
        self.methods()
            .iter()
            .filter(|m| m.visibility == VisibilityModifier::Public)
    }

    pub fn method_named_as(
        &self,
        name: SymbolPointer,
        visibility: VisibilityModifier,
    ) -> Option<DeclarationId<HirFunctionDeclaration>> {
        self.methods()
            .iter()
            .find_map(|m| (m.data.0 == name && m.visibility == visibility).then_some(m.data.1))
    }
}

impl HirViewer<'_, DedupPoolId<TupleType>> {
    pub fn fields(&self) -> &[DedupPoolId<HirType>] {
        &self.hir.types_module[self.data].fields
    }
}
