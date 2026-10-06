use dashmap::mapref::one::MappedRef;
use module_loader::FileId;

use crate::{
    DeclarationId, HirFunctionDeclaration, VariableId, file::HirFile, helpers::HirViewer,
    term::TermId,
};

impl HirViewer<'_, DeclarationId<HirFunctionDeclaration>> {
    pub fn get_argument(&self, arg: u8) -> Option<(VariableId, TermId)> {
        self.get_argument_type(arg)
            .map(|ty| (VariableId::function(self.data, arg as u16), ty))
    }
    pub fn get_argument_type(&self, arg: u8) -> Option<TermId> {
        self.ty()
            .is_function()
            .expect("Expected Function to have a function type")
            .0
            .get(arg as usize)
            .cloned()
    }

    pub fn return_type(&self) -> TermId {
        self.ty()
            .is_function()
            .expect("Expected Function to have a function type")
            .1
    }

    pub fn ty(&self) -> HirViewer<'_, TermId> {
        let ty = self.hir.get_function(self.data).ty;
        self.new_with(ty)
    }
    pub fn raw_declaration(&self) -> MappedRef<'_, FileId, HirFile, HirFunctionDeclaration> {
        self.hir.get_function(self.data)
    }
}
