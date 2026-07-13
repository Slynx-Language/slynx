use common::pool::DedupPoolId;

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
}

impl HirViewer<'_, DedupPoolId<TupleType>> {
    pub fn fields(&self) -> &[DedupPoolId<HirType>] {
        &self.hir.types_module[self.data].fields
    }
}
