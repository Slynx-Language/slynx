use common::{FrontendSymbol, SymbolPointer, pool::DedupPoolId};

use crate::{
    DeclarationId, HirFunctionDeclaration, HirType, VariableId,
    helpers::HirViewer,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};

impl HirViewer<'_, AnyDeclarationId> {
    pub fn name(&self) -> SymbolPointer<FrontendSymbol> {
        let file_id = self.data.file_id;
        match self.data.local_id {
            AnyLocalDeclarationId::Alias(local_id) => {
                self.hir
                    .get_alias(DeclarationId::new(file_id, local_id))
                    .name
            }
            AnyLocalDeclarationId::Function(local_id) => {
                self.hir
                    .get_function(DeclarationId::new(file_id, local_id))
                    .name
            }
            AnyLocalDeclarationId::Object(local_id) => {
                self.hir
                    .get_object(DeclarationId::new(file_id, local_id))
                    .name
            }
            AnyLocalDeclarationId::Component(local_id) => {
                self.hir
                    .get_component(DeclarationId::new(file_id, local_id))
                    .name
            }
            AnyLocalDeclarationId::Static(local_id) => {
                self.hir
                    .get_static(DeclarationId::new(file_id, local_id))
                    .name
            }
            AnyLocalDeclarationId::Style(local_id) => {
                self.hir
                    .get_style(DeclarationId::new(file_id, local_id))
                    .name
            }
        }
    }
}

impl HirViewer<'_, DeclarationId<HirFunctionDeclaration>> {
    pub fn get_argument(&self, arg: u8) -> Option<(VariableId, DedupPoolId<HirType>)> {
        self.get_argument_type(arg)
            .map(|ty| (VariableId::function(self.data, arg as u16), ty))
    }
    pub fn get_argument_type(&self, arg: u8) -> Option<DedupPoolId<HirType>> {
        self.ty()
            .is_function()
            .expect("Expected Function to have a function type")
            .arguments()
            .get(arg as usize)
            .cloned()
    }

    pub fn return_type(&self) -> DedupPoolId<HirType> {
        self.ty()
            .is_function()
            .expect("Expected Function to have a function type")
            .return_type()
    }

    pub fn ty(&self) -> HirViewer<'_, DedupPoolId<HirType>> {
        let ty = self.hir.get_function(self.data).ty;
        self.new_with(ty)
    }
}
