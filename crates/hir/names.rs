use crate::hir::{
    Result, SlynxHir, TypeId, VariableId,
    error::{HIRError, InvalidTypeReason},
    model::HirType,
};

use common::{
    SymbolPointer,
    ast::{GenericIdentifier, Span},
};
//file specific to implement things related to name resolution
impl SlynxHir {
    ///Retrieves the pointer(simply a symbol) of the provided `name`.
    pub fn get_symbol_of(&self, name: &str) -> Option<SymbolPointer> {
        self.modules.retrieve_symbol(name)
    }

    ///Since when a object is defined, its generated as an unnamed type, and has got a reference to it, this retrieves the inner layout of the object
    pub fn get_object_type_from_name(&mut self, name: &str, span: &Span) -> Result<&HirType> {
        let name_symbol = self.modules.intern_name(name);

        if let Some(ref_id) = self.modules.types_module.get_id(&name_symbol)
            && let HirType::Reference { rf, .. } = self.modules.types_module.get_type(ref_id)
        {
            Ok(self.modules.types_module.get_type(rf))
        } else {
            Err(HIRError::invalid_type(
                name_symbol,
                InvalidTypeReason::IncorrectUsage,
                *span,
            ))
        }
    }

    /// Resolves the [`TypeId`] for the given plain type name string.
    ///
    /// Handles built-in names (`int`, `float`, `str`, `bool`, `void`, `Component`) directly,
    /// and falls back to the module's type registry for user-defined types.
    pub fn get_typeid_of_name(&mut self, name: &str, span: &Span) -> Result<TypeId> {
        match name {
            "Component" => Ok(self.component_type()),
            "()" | "void" => Ok(self.void_type()),
            "bool" => Ok(self.bool_type()),
            "int" => Ok(self.int32_type()),
            "float" => Ok(self.float32_type()),
            "str" => Ok(self.str_type()),
            _ => self.modules.find_type_by_name(name, span).cloned(),
        }
    }

    /// Resolves the [`TypeId`] for a [`GenericIdentifier`], handling tuple and generic types.
    pub fn get_typeid_of_generic(&mut self, gener: &GenericIdentifier) -> Result<TypeId> {
        match gener.identifier.as_str() {
            "tuple" if let Some(ref types) = gener.generic => {
                let fields = types
                    .iter()
                    .map(|field| self.get_typeid_of_generic(field))
                    .collect::<Result<Vec<_>>>()?;
                Ok(self.add_tuple_type(fields))
            }
            "tuple" if let None = &gener.generic => {
                let interned = self.modules.intern_name(&gener.identifier);
                Err(HIRError::name_unrecognized(interned, gener.span))
            }
            "()" => Ok(self.void_type()),
            _ if let Some(ref generics) = gener.generic => {
                let gen_ids = generics
                    .iter()
                    .map(|generic| self.get_typeid_of_generic(generic))
                    .collect::<Result<Vec<_>>>()?;
                let base_id = self.get_typeid_of_name(&gener.identifier, &gener.span)?;
                Ok(self.add_unnamed_type(HirType::new_generic_ref(base_id, gen_ids)))
            }
            _ => self.get_typeid_of_name(&gener.identifier, &gener.span),
        }
    }

    ///Tries to retrieve a variable with the provided `name` on the current active scope
    pub fn get_variable(&mut self, symbol: SymbolPointer, span: &Span) -> Result<VariableId> {
        if let Some(variable) = self.modules.find_variable(symbol) {
            Ok(variable)
        } else {
            Err(HIRError::name_unrecognized(symbol, *span))
        }
    }
    ///Creates a mutable variable with the given `name` and `ty`
    pub fn create_mutable_variable(
        &mut self,
        symbol: SymbolPointer,
        ty: TypeId,
        span: &Span,
    ) -> Result<VariableId> {
        self.modules.create_variable(symbol, true, ty, span)
    }
    ///Creates a imutable variable with the given `name` and `ty`
    pub fn create_variable(
        &mut self,
        symbol: SymbolPointer,
        ty: TypeId,
        span: &Span,
    ) -> Result<VariableId> {
        self.modules.create_variable(symbol, false, ty, span)
    }
}
