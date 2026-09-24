use crate::{
    DeclarationId, HirFunctionDeclaration, SymbolPointer, helpers::HirViewer, term::TermId,
};

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

    pub fn type_viewer(&self) -> (&[TermId], TermId) {
        let ty = self.hir.get_function(self.data).ty;
        let viewer = self.hir.view(ty);
        if let Some((args, ret)) = viewer.is_function() {
            (args, ret)
        } else {
            panic!("Type of function is not HirType::Function. internal error during hir creation");
        }
    }
}
