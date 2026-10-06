use common::{VisibilityModifier, pool::DedupPoolId};

use crate::{
    DeclarationId, EnumType, EnumVariantType, HirFunctionDeclaration, StructField, StructMethod,
    StructType, SymbolPointer, TupleType,
    helpers::{HirViewer, Visible},
    term::TermId,
};

impl HirViewer<'_, DedupPoolId<StructType>> {
    pub fn name(&self) -> SymbolPointer {
        self.hir.types.get_struct_name(self.data)
    }

    pub fn fields(&self) -> &[Visible<StructField>] {
        self.hir.types.get_struct_fields(self.data)
    }

    pub fn methods(&self) -> &[Visible<StructMethod>] {
        &self.hir.types[self.data].methods
    }
    pub fn public_methods(&self) -> impl Iterator<Item = &Visible<StructMethod>> {
        self.methods()
            .iter()
            .filter(|m| m.visibility == VisibilityModifier::Public)
    }

    pub fn method_named_as(
        &self,
        name: SymbolPointer,
        visibility: VisibilityModifier,
    ) -> Option<DeclarationId<HirFunctionDeclaration>> {
        self.methods().iter().find_map(|m| {
            (m.data.name == name && m.visibility == visibility).then_some(m.data.target)
        })
    }
}

impl HirViewer<'_, DedupPoolId<TupleType>> {
    pub fn fields(&self) -> &[TermId] {
        &self.hir.types[self.data].fields
    }
}

impl HirViewer<'_, DedupPoolId<EnumType>> {
    pub fn name(&self) -> SymbolPointer {
        self.hir.types.get_enum_name(self.data)
    }

    pub fn variants(&self) -> &[EnumVariantType] {
        self.hir.types.get_enum_variants(self.data)
    }

    ///Finds the variant with the given `name` on this enum.
    pub fn find_variant(&self, name: SymbolPointer) -> Option<usize> {
        self.hir.types.find_enum_variant(self.data, name)
    }
}
