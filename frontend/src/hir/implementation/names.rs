use crate::hir::{Result, SlynxHir, TypeId, error::HIRError, model::HirType};

use common::ast::{GenericIdentifier, Span};
impl SlynxHir {
    ///Creates a type with the provided `name` and `ty`. Returns it's ID
    pub fn assing_type(&mut self, type_id: TypeId, ty: HirType) {
        self.types.insert(type_id, ty);
    }

    ///Retrieves the type of the provided `name` but in the global scope
    pub fn retrieve_type_of_name(
        &mut self,
        name: &GenericIdentifier,
        span: &Span,
    ) -> Result<HirType> {
        let name_id = self.modules.intern_name(&name.identifier);
        match HirType::new(&name.identifier) {
            Some(value) => Ok(value),
            _ if let Some(ty) = self.get_typeid_from_name(&name_id) => Ok(HirType::new_ref(*ty)),
            _ => Err(HIRError::name_unrecognized(name_id, *span)),
        }
    }
    ///Tries to retrieve the type and `TypeId` of the provided `name` in the global scope
    pub fn retrieve_information_of_type(
        &mut self,
        name: &str,
        span: &Span,
    ) -> Result<(TypeId, &HirType)> {
        let name_symbol = self.modules.intern_name(name);
        match () {
            _ if let Ok(id) = self.get_typeid_of_name(name, span) => Ok((id, self.get_type(&id))),
            _ if let Some(id) = self.get_typeid_from_name(&name_symbol) => {
                Ok((*id, self.get_type(id)))
            }
            _ => Err(HIRError::name_unrecognized(name_symbol, *span)),
        }
    }
    ///Retrieves the type of the provided `name` but in the global scope. The difference of a 'named' to a 'name' is that this function
    ///tries to the the provided `name` as some identifier to something, and the name version does so after checking if the provided name itself
    ///is a type
    pub fn retrieve_type_of_named(&mut self, name: &str, span: &Span) -> Result<&HirType> {
        let name_id = self.modules.intern_name(name);
        match self.get_type_from_name(&name_id) {
            Some(ty) => Ok(ty),
            _ => Err(HIRError::name_unrecognized(name_id, *span)),
        }
    }

    pub fn retrieve_ref_to_type(&mut self, name: &str, span: &Span) -> Result<&mut HirType> {
        let name_symbol = self.modules.intern_name(name);
        match self.get_type_mut_from_name(&name_symbol) {
            Some(ty) => Ok(ty),
            _ => Err(HIRError::name_unrecognized(name_symbol, *span)),
        }
    }
}
