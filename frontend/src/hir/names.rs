use color_eyre::eyre::Result;

use crate::hir::{
    SlynxHir, TypeId, VariableId,
    error::{HIRError, HIRErrorKind, InvalidTypeReason},
    symbols::SymbolPointer,
    types::HirType,
};

use common::ast::{GenericIdentifier, Span};
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
            "()" => Ok(self.types_module.void_id()),
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

    pub fn get_typeid_of_generic(&mut self, gener: &GenericIdentifier) -> Result<TypeId> {
        match gener.identifier.as_str() {
            "tuple" => {
                if let Some(types) = &gener.generic {
                    let mut fields = Vec::with_capacity(types.len());
                    for t in types {
                        fields.push(self.get_typeid_of_generic(t)?);
                    }
                    Ok(self.types_module.add_tuple_type(fields))
                } else {
                    Err(HIRError {
                        kind: HIRErrorKind::NameNotRecognized(gener.identifier.clone()),
                        span: gener.span.clone(),
                    }
                    .into())
                }
            }
            "()" => Ok(self.types_module.void_id()),
            _ => {
                if let Some(gens) = &gener.generic {
                    let gen_ids = {
                        let mut tmp = Vec::with_capacity(gens.len());
                        for g in gens {
                            tmp.push(self.get_typeid_of_generic(g)?);
                        }
                        tmp
                    };
                    let base_id = self.get_typeid_of_name(&gener.identifier, &gener.span)?;
                    Ok(self.types_module.insert_unnamed_type(HirType::Reference {
                        rf: base_id,
                        generics: gen_ids,
                    }))
                } else {
                    self.get_typeid_of_name(&gener.identifier, &gener.span)
                }
            }
        }
    }

    ///Creates a new variable with the provided `name` on the current scope and returns its id
    pub fn create_variable(&mut self, name: &str, ty: TypeId, mutable: bool) -> VariableId {
        let ptr = self.symbols_module.intern(name);
        let v = VariableId::new();
        self.scope_module.insert_name(ptr, v, mutable);
        self.types_module.insert_variable(v, ty);
        self.variable_names.insert(v, ptr);
        v
    }

    pub fn variable_names(&self) -> &std::collections::HashMap<VariableId, SymbolPointer> {
        &self.variable_names
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
