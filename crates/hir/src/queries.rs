use common::{Span, VisibilityModifier};

use crate::{
    DeclarationId, HIRError, HirDeclaration, HirType, Result, SlynxHir, SymbolPointer, TypeId,
    context::{TypeReader, TypeWriter},
    file::HirFile,
    module_loader::FileId,
};
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

impl SlynxHir {
    ///Interns the given `s` string and returns its logical pointer
    pub fn intern_name(&self, s: &str) -> SymbolPointer {
        self.symbols_resolver.intern(s)
    }

    ///Retrieves the symbol pointer for the given `s` if it exists, thus, was internalized
    pub fn retrieve_symbol(&self, s: &str) -> Option<SymbolPointer> {
        self.symbols_resolver.retrieve(s)
    }
    ///Gets the HIR type of the given `ty`
    pub fn get_type(&self, ty: &TypeId) -> TypeReader<'_> {
        self.types_module.get_type(ty)
    }
    ///Gets the HIR type of the given `ty`
    pub fn get_type_mut(&self, ty: TypeId) -> TypeWriter<'_> {
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
        self.get_name(ptr)
    }
    ///Retrieves the type of something by asserting the provided `ref_ty` is a reference type to it
    pub fn get_type_from_ref(&self, ref_ty: TypeId, span: &Span) -> Result<TypeReader<'_>> {
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
                .ok_or(HIRError::type_unrecognized(name, *span)),
        }
    }

    pub fn get_name_of_type(&self, ty: TypeId) -> Option<SymbolPointer> {
        self.types_module.get_type_name(&ty)
    }

    pub fn get_file(&self, id: FileId) -> RwLockReadGuard<'_, HirFile> {
        self.files[id.as_raw() as usize].read()
    }
    pub fn get_file_mut(&self, id: FileId) -> RwLockWriteGuard<'_, HirFile> {
        self.files[id.as_raw() as usize].write()
    }

    pub fn find_declaration(
        &self,
        id: DeclarationId,
    ) -> parking_lot::MappedRwLockReadGuard<'_, HirDeclaration> {
        parking_lot::RwLockReadGuard::map(self.files[id.file_id.as_raw() as usize].read(), |f| {
            &f.declarations.declarations[id.local_id.as_raw()]
        })
    }

    pub fn find_declaration_by_name(
        &self,
        name: &SymbolPointer,
        span: Span,
    ) -> Result<(DeclarationId, TypeId)> {
        // 1. Check import aliases first
        for file in &self.files {
            if let Some(alias_data) = file.read().declarations.get_import_alias(name) {
                return Ok(alias_data);
            }
        }
        // 2. Check regular declarations
        let mut result: Option<(DeclarationId, TypeId)> = None;
        for file in &self.files {
            let file = file.read();
            if let Some((local, ty)) = file.declarations.get_declaration_data_by_name(name) {
                if let Some((decl, _)) = result {
                    return Err(HIRError::ambiguous_declaration(
                        *name,
                        decl.file_id,
                        file.file,
                        span,
                    ));
                }
                let id = DeclarationId::new(file.file, local);
                result = Some((id, ty));
            }
        }

        result.ok_or_else(|| HIRError::name_unrecognized(*name, span))
    }

    /// Looks up a declaration by name, but only within the given set of files.
    pub fn find_declaration_in_files(
        &self,
        name: &SymbolPointer,
        file_ids: &[FileId],
        span: Span,
    ) -> Result<(DeclarationId, TypeId)> {
        for &fid in file_ids {
            let file = self.get_file(fid);
            // Check import aliases registered in this file first
            if let Some(alias_data) = file.declarations.get_import_alias(name) {
                return Ok(alias_data);
            }
            if let Some((local, ty)) = file.declarations.get_declaration_data_by_name(name) {
                if file.declarations.get_visibility(local) != VisibilityModifier::Public {
                    return Err(HIRError::name_unrecognized(*name, span));
                }
                let id = DeclarationId::new(file.file, local);
                return Ok((id, ty));
            }
        }
        Err(HIRError::name_unrecognized(*name, span))
    }
    pub fn type_of_intrinsic(&self, name: &str) -> TypeId {
        let id = self.lang_items.get(name);
        self.get_declaration_type(id)
    }

    /// Recursively flattens a HIR type to its primitive components.
    /// A struct `Color { inner: int }` flattens to `[int]`.
    /// A struct `Border { color: Color, width: int, radius: int }` flattens to `[int, int, int]`.
    pub fn flatten_type(&self, ty: TypeId) -> Vec<TypeId> {
        match &*self.get_type(&ty) {
            HirType::Int | HirType::Float | HirType::Bool | HirType::Str => vec![ty],
            HirType::Struct { fields } => {
                fields.iter().flat_map(|f| self.flatten_type(*f)).collect()
            }
            HirType::Reference { rf, .. } => self.flatten_type(*rf),
            _ => vec![ty],
        }
    }
}
