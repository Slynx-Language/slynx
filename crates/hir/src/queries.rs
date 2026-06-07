use common::Span;

use crate::{
    DeclarationId, HIRError, HirType, Result, SlynxHir, SymbolPointer, TypeId, file::HirFile,
    module_loader::FileId,
};

impl SlynxHir {
    ///Interns the given `s` string and returns its logical pointer
    pub fn intern_name(&mut self, s: &str) -> SymbolPointer {
        self.symbols_resolver.intern(s)
    }

    ///Retrieves the symbol pointer for the given `s` if it exists, thus, was internalized
    pub fn retrieve_symbol(&self, s: &str) -> Option<SymbolPointer> {
        self.symbols_resolver.retrieve(s).cloned()
    }
    ///Gets the HIR type of the given `ty`
    pub fn get_type(&self, ty: &TypeId) -> &HirType {
        self.types_module.get_type(ty)
    }
    ///Gets the HIR type of the given `ty`
    pub fn get_type_mut(&mut self, ty: &TypeId) -> &mut HirType {
        self.types_module.get_type_mut(ty)
    }

    pub fn get_name(&self, name: SymbolPointer) -> &str {
        self.symbols_resolver.get_name(name)
    }

    pub fn get_declaration_name(&self, id: DeclarationId) -> &str {
        let ty = self.get_declaration_type(id);
        let ptr = self
            .types_module
            .get_type_name(&ty)
            .expect("Declaration should contain a name");
        self.get_name(*ptr)
    }
    ///Retrieves the type of something by asserting the provided `ref_ty` is a reference type to it
    pub fn get_type_from_ref(&self, ref_ty: TypeId, span: &Span) -> Result<&HirType> {
        let ty = self.types_module.get_type_from_ref(ref_ty, span)?;
        Ok(self.get_type(&ty))
    }
    /// Resolves the [`TypeId`] for the given plain type name string.
    ///
    /// Handles built-in names (`int`, `float`, `str`, `bool`, `void`, `Component`) directly,
    /// and falls back to the module's type registry for user-defined types.
    pub fn get_type_of_name(&self, name: SymbolPointer, span: &Span) -> Result<TypeId> {
        let name_ref = self.get_name(name);
        match name_ref {
            "Component" => Ok(self.component_type()),
            "()" | "void" => Ok(self.void_type()),
            "bool" => Ok(self.bool_type()),
            "int" => Ok(self.int32_type()),
            "float" => Ok(self.float32_type()),
            "str" => Ok(self.str_type()),
            _ => self
                .types_module
                .get_id(&name)
                .cloned()
                .ok_or(HIRError::type_unrecognized(name, *span)),
        }
    }

    pub fn get_name_of_type(&self, ty: TypeId) -> Option<SymbolPointer> {
        self.types_module.get_type_name(&ty).cloned()
    }

    pub fn get_file(&self, id: FileId) -> &HirFile {
        &self.files[id.as_raw() as usize]
    }
    pub fn get_file_mut(&mut self, id: FileId) -> &mut HirFile {
        &mut self.files[id.as_raw() as usize]
    }
}
