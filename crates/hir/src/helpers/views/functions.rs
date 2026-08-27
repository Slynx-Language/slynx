use common::pool::DedupPoolId;

use crate::{
    DeclarationId, FunctionType, HirFunctionDeclaration, HirType, SymbolPointer, helpers::HirViewer,
};

impl HirViewer<'_, DedupPoolId<FunctionType>> {
    pub fn arguments(&self) -> &[DedupPoolId<HirType>] {
        &self.hir.types_module[self.data].args
    }
    pub fn return_type(&self) -> DedupPoolId<HirType> {
        self.hir.types_module[self.data].ret
    }
}

impl HirViewer<'_, DeclarationId<HirFunctionDeclaration>> {
    pub fn name(&self) -> SymbolPointer {
        self.hir.get_function(self.data).name
    }
    pub fn generic(&self, generic: usize) -> Option<SymbolPointer> {
        self.hir
            .get_function(self.data)
            .generics
            .get(generic)
            .cloned()
    }
    pub fn generic_count(&self) -> usize {
        self.hir.get_function(self.data).generics.len()
    }
    pub fn type_viewer(&self) -> HirViewer<'_, DedupPoolId<FunctionType>> {
        let ty = self.hir.get_function(self.data).ty;
        let viewer = self.hir.view(ty);
        if let Some(func_viewer) = viewer.is_function() {
            let data = func_viewer.data;
            HirViewer {
                hir: self.hir,
                data,
            }
        } else {
            panic!("Type of function is not HirType::Function. internal error during hir creation");
        }
    }
}
