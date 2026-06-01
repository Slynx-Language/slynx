use std::collections::HashSet;

use common::Span;

use crate::{DeclarationId, HIRError, HirType, Result, SlynxHir, SymbolPointer, TypeId};

impl SlynxHir {
    ///Gets the HIR type of the given `ty`
    pub fn get_type(&self, ty: &TypeId) -> &HirType {
        self.modules.types_module.get_type(ty)
    }
    ///Gets the HIR type of the given `ty`
    pub fn get_type_mut(&mut self, ty: &TypeId) -> &mut HirType {
        self.modules.types_module.get_type_mut(ty)
    }

    pub fn get_name(&self, name: SymbolPointer) -> &str {
        self.modules.symbols_resolver.get_name(name)
    }

    pub fn get_declaration_name(&self, id: DeclarationId) -> &str {
        let ty = self.modules.declarations_module.get_declaration_type(id);
        let ptr = self
            .modules
            .types_module
            .get_type_name(&ty)
            .expect("Declaration should contain a name");
        self.get_name(*ptr)
    }
    ///Retrieves the type of something by asserting the provided `ref_ty` is a reference type to it
    pub fn get_type_from_ref(&self, mut ref_ty: TypeId, span: &Span) -> Result<&HirType> {
        let mut visited = HashSet::new();
        while let HirType::Reference { rf, .. } = self.get_type(&ref_ty) {
            if !visited.insert(ref_ty) {
                let name = self
                    .get_name_of_type(ref_ty)
                    .expect("Type should contain a name");
                return Err(HIRError::recursive(name, *span));
            }
            ref_ty = *rf;
        }
        Ok(self.get_type(&ref_ty))
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
                .modules
                .types_module
                .get_id(&name)
                .cloned()
                .ok_or(HIRError::type_unrecognized(name, *span)),
        }
    }

    pub fn get_name_of_type(&self, ty: TypeId) -> Option<SymbolPointer> {
        self.modules.types_module.get_type_name(&ty).cloned()
    }
}
