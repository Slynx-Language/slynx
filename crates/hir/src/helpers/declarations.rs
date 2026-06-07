use crate::{DeclarationId, HirType, SlynxHir, SymbolPointer, TypeId, module_loader::FileId};

impl SlynxHir {
    ///Creates an type alias with the given `name`. Its initial type is `infer`. Because of hoisting, and so, the type this refers to might be defined after it
    pub(crate) fn create_empty_alias(
        &mut self,
        aliasname: SymbolPointer,
        file: FileId,
    ) -> DeclarationId {
        let ty = self.types_module.create_type(aliasname, HirType::Infer);
        let local_id = self
            .get_file_mut(file)
            .declarations
            .register_declaration_metadata(aliasname, ty);
        DeclarationId::new(file, local_id)
    }

    pub fn get_declaration_type(&self, id: DeclarationId) -> TypeId {
        self.get_file(id.file_id)
            .declarations
            .get_declaration_type(id.local_id)
    }
}
