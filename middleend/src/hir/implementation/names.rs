use color_eyre::eyre::Result;

use crate::{
    hir::{
        DeclarationId, SlynxHir, TypeId,
        error::{HIRError, HIRErrorKind},
        types::HirType,
    },
    
};

use common::ast::{GenericIdentifier, Span};
impl SlynxHir {
    pub fn create_declaration(&mut self, name: &str, ty: TypeId) -> DeclarationId {
        let ptr = self.symbols_module.intern(name);
        self.declarations_module.create_declaration(ptr, ty)
    }

    ///Creates a type with the provided `name` and `ty`. Returns it's ID
    pub fn assing_type(&mut self, type_id: TypeId, ty: HirType) {
        self.types.insert(type_id, ty);
    }

    ///Retrieves the type of the provided `name` but in the global scope
    pub fn retrieve_type_of_name(&self, name: &GenericIdentifier, span: &Span) -> Result<HirType> {
        match HirType::new(name) {
            Ok(value) => Ok(value),
            Err(_) => {
                if let Some(name_id) = self.symbols_module.retrieve(&name.identifier)
                    && let Some(ty) = self.types_module.get_id(name_id)
                {
                    Ok(HirType::Reference {
                        rf: *ty,
                        generics: Vec::new(),
                    })
                } else {
                    Err(HIRError {
                        kind: HIRErrorKind::NameNotRecognized(name.to_string()),
                        span: span.clone(),
                    }
                    .into())
                }
            }
        }
    }
    ///Tries to retrieve the type and `TypeId` of the provided `name` in the global scope
    pub fn retrieve_information_of_type(
        &self,
        name: &str,
        span: &Span,
    ) -> Result<(TypeId, &HirType)> {
        if let Ok(id) = self.get_typeid_of_name(name, span) {
            Ok((id, self.types_module.get_type(&id)))
        } else if let Some(name_id) = self.symbols_module.retrieve(name)
            && let Some(id) = self.types_module.get_id(name_id)
        {
            let ty = self.types_module.get_type(id);
            Ok((*id, ty))
        } else {
            Err(HIRError {
                kind: HIRErrorKind::NameNotRecognized(name.to_string()),
                span: span.clone(),
            }
            .into())
        }
    }
    ///Retrieves the type of the provided `name` but in the global scope. The difference of a 'named' to a 'name' is that this function
    ///tries to the the provided `name` as some identifier to something, and the name version does so after checking if the provided name itself
    ///is a type
    pub fn retrieve_type_of_named(&self, name: &str, span: &Span) -> Result<&HirType, HIRError> {
        if let Some(name_id) = self.symbols_module.retrieve(name)
            && let Some(ty) = self.types_module.get_type_from_name(name_id)
        {
            Ok(ty)
        } else {
            Err(HIRError {
                kind: HIRErrorKind::NameNotRecognized(name.to_string()),
                span: span.clone(),
            })
        }
    }

    pub fn retrieve_ref_to_type(
        &mut self,
        name: &str,
        span: &Span,
    ) -> Result<&mut HirType, HIRError> {
        if let Some(name_id) = self.symbols_module.retrieve(name)
            && let Some(ty) = self.types_module.get_type_from_name_mut(name_id)
        {
            Ok(ty)
        } else {
            Err(HIRError {
                kind: HIRErrorKind::NameNotRecognized(name.to_string()),
                span: span.clone(),
            })
        }
    }
}
