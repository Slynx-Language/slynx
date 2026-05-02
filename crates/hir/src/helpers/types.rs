use common::SymbolPointer;

use crate::{SlynxHir, TypeId, VariableId, model::HirType};

impl SlynxHir {
    /// Retrieves the id of the `infer` type.
    pub fn infer_type(&self) -> TypeId {
        self.modules.types_module.infer_id()
    }
    /// Retrieves the id of the `int32`
    pub fn int32_type(&self) -> TypeId {
        self.modules.types_module.int_id()
    }
    ///Retrieves the id of type `float32`
    pub fn float32_type(&self) -> TypeId {
        self.modules.types_module.float_id()
    }

    ///Retrieves the id of type `void` or `()`
    pub fn void_type(&self) -> TypeId {
        self.modules.types_module.void_id()
    }

    ///Retrieves the id of type `bool`
    pub fn bool_type(&self) -> TypeId {
        self.modules.types_module.bool_id()
    }

    ///Retrieves the id of type `str`
    pub fn str_type(&self) -> TypeId {
        self.modules.types_module.str_id()
    }

    ///Retrieves the id of type `Component`
    pub fn component_type(&self) -> TypeId {
        self.modules.types_module.generic_component_id()
    }
    ///Gets the HIR type of the given `ty`
    pub fn get_type(&self, ty: &TypeId) -> &HirType {
        self.modules.types_module.get_type(ty)
    }

    ///Gets the HIR type of the given `ty`
    pub fn get_type_mut(&mut self, ty: &TypeId) -> &mut HirType {
        self.modules.types_module.get_type_mut(ty)
    }

    ///Creates a new tuple type with the given `fields` and returns its typeid
    pub fn add_tuple_type(&mut self, fields: Vec<TypeId>) -> TypeId {
        self.modules.types_module.add_tuple_type(fields)
    }

    ///Creates a the given `ty` on the hir and returns its id. doesnt map a name to it
    pub fn add_unnamed_type(&mut self, ty: HirType) -> TypeId {
        self.modules.types_module.insert_unnamed_type(ty)
    }
    /// Inserts a named type into the HIR and returns its [`TypeId`].
    pub fn add_type(&mut self, name: SymbolPointer, ty: HirType) -> TypeId {
        self.modules.types_module.insert_type(name, ty)
    }

    /// Returns the [`HirType`] associated with the given name, if it exists.
    pub fn get_type_from_name(&self, name: &SymbolPointer) -> Option<&HirType> {
        self.modules.types_module.get_type_from_name(name)
    }

    /// Returns a mutable reference to the [`HirType`] associated with the given name, if it exists.
    pub fn get_type_mut_from_name(&mut self, name: &SymbolPointer) -> Option<&mut HirType> {
        self.modules.types_module.get_type_from_name_mut(name)
    }

    /// Returns the [`TypeId`] associated with the given name, if it exists.
    pub fn get_typeid_from_name(&self, name: &SymbolPointer) -> Option<&TypeId> {
        self.modules.types_module.get_id(name)
    }

    /// Returns the field layout (as a slice of symbol pointers) for the object with the given [`TypeId`].
    pub fn retrieve_object_fields(&self, ty: TypeId) -> Option<&[SymbolPointer]> {
        self.modules.declarations_module.retrieve_object_body(ty)
    }

    /// Returns the [`TypeId`] of the given variable, if it exists.
    pub fn get_variable_type(&self, ty: VariableId) -> Option<&TypeId> {
        self.modules.types_module.get_variable(&ty)
    }
}
