use color_eyre::eyre::Result;

use crate::{
    frontend::parser::ast::Span,
    middleend::hir::{
        SlynxHir, TypeId, VariableId,
        error::{HIRError, HIRErrorKind, InvalidTypeReason},
        symbols::SymbolPointer,
        types::HirType,
    },
};

//file specific to implement things related to name resolution
impl SlynxHir {
    pub fn insert_name(&mut self, name: &str) -> super::symbols::SymbolPointer {
        self.symbols_module.intern(name)
    }
    ///Retrieves the pointer(simply a symbol) of the provided `name`.
    pub fn get_symbol_of(&self, name: &str, span: &Span) -> Result<SymbolPointer> {
        self.symbols_module.retrieve(name).cloned().ok_or(
            HIRError {
                kind: HIRErrorKind::NameNotRecognized(name.to_string()),
                span: span.clone(),
            }
            .into(),
        )
    }

    ///Retrieves the type of something by knowing the provided `ref_ty` is a reference to it
    pub fn get_type_from_ref(&self, ref_ty: TypeId) -> &HirType {
        if let HirType::Reference { rf, .. } = self.types_module.get_type(&ref_ty) {
            self.types_module.get_type(rf)
        } else {
            unreachable!("The provided ref_ty should be of type Reference");
        }
    }

    ///Since when a object is defined, its generated as an unnamed type, and has got a reference to it, this retrieves the inner layout of the object
    pub fn get_object_type_from_name(&self, name: &str, span: &Span) -> Result<&HirType> {
        if let Some(symbol) = self.symbols_module.retrieve(name) {
            if let Some(ref_id) = self.types_module.get_id(symbol)
                && let HirType::Reference { rf, .. } = self.types_module.get_type(ref_id)
            {
                Ok(self.types_module.get_type(rf))
            } else {
                Err(HIRError {
                    kind: HIRErrorKind::InvalidType {
                        ty: name.to_string(),
                        reason: InvalidTypeReason::IncorrectUsage,
                    },
                    span: span.clone(),
                }
                .into())
            }
        } else {
            Err(HIRError {
                kind: HIRErrorKind::NameNotRecognized(name.to_string()),
                span: span.clone(),
            }
            .into())
        }
    }

    pub fn get_typeid_of_name(&self, name: &str, span: &Span) -> Result<TypeId> {
        match name {
            "Component" => Ok(self.types_module.generic_component_id()),
            "void" => Ok(self.types_module.void_id()),
            "bool" => Ok(self.types_module.bool_id()),
            "int" => Ok(self.types_module.int_id()),
            "float" => Ok(self.types_module.float_id()),
            "str" => Ok(self.types_module.str_id()),
            _ => {
                let temp = self
                    .symbols_module
                    .retrieve(name)
                    .and_then(|id| self.types_module.get_id(id).cloned());
                temp.ok_or(
                    HIRError {
                        kind: HIRErrorKind::NameNotRecognized(name.to_string()),
                        span: span.clone(),
                    }
                    .into(),
                )
            }
        }
    }

    ///Creates a new variable with the provided `name` on the current scope and returns its id
    pub fn create_variable(&mut self, name: &str, ty: TypeId, mutable: bool) -> VariableId {
        let ptr = self.symbols_module.intern(name);
        let v = VariableId::new();
        self.scope_module.insert_name(ptr, v, mutable);
        self.types_module.insert_variable(v, ty);
        v
    }
    ///Tries to retrieve a variable with the provided `name` on the current active scope
    pub fn get_variable(&mut self, name: &str, span: &Span) -> Result<&VariableId> {
        if let Some(symbol) = self.symbols_module.retrieve(name)
            && let Some(variable) = self.scope_module.retrieve_name(symbol)
        {
            Ok(variable)
        } else {
            Err(HIRError {
                kind: HIRErrorKind::NameNotRecognized(name.to_string()),
                span: span.clone(),
            }
            .into())
        }
    }
    pub fn define_type(&mut self, name: SymbolPointer, ty: HirType) -> TypeId {
        self.types_module.insert_type(name, ty)
    }
}
