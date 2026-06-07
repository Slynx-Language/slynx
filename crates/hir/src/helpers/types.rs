use crate::{SlynxHir, SymbolPointer, TypeId, VariableId, model::HirType};

impl SlynxHir {
    /// Retrieves the id of the `infer` type.
    pub fn infer_type(&self) -> TypeId {
        self.types_module.infer_id()
    }
    /// Retrieves the id of the `int32`
    pub fn int32_type(&self) -> TypeId {
        self.types_module.int_id()
    }
    ///Retrieves the id of type `float32`
    pub fn float32_type(&self) -> TypeId {
        self.types_module.float_id()
    }

    ///Retrieves the id of type `void` or `()`
    pub fn void_type(&self) -> TypeId {
        self.types_module.void_id()
    }

    ///Retrieves the id of type `bool`
    pub fn bool_type(&self) -> TypeId {
        self.types_module.bool_id()
    }

    ///Retrieves the id of type `str`
    pub fn str_type(&self) -> TypeId {
        self.types_module.str_id()
    }

    ///Retrieves the id of type `Component`
    pub fn component_type(&self) -> TypeId {
        self.types_module.generic_component_id()
    }
    ///Creates a new tuple type with the given `fields` and returns its typeid
    pub fn create_tuple_type(&mut self, fields: Vec<TypeId>) -> TypeId {
        self.types_module.create_tuple_type(fields)
    }

    ///Creates a the given `ty` on the hir and returns its id. doesnt map a name to it
    pub fn create_unnamed_type(&mut self, ty: HirType) -> TypeId {
        self.types_module.create_unnamed_type(ty)
    }
    /// Inserts a named type into the HIR and returns its [`TypeId`].
    pub fn create_type(&mut self, name: SymbolPointer, ty: HirType) -> TypeId {
        self.types_module.create_type(name, ty)
    }

    /// Returns the field layout (as a slice of symbol pointers) for the object with the given [`TypeId`].
    pub fn get_object_fields(&self, ty: TypeId) -> Option<&[SymbolPointer]> {
        self.declarations_module.get_object_body(ty)
    }

    /// Returns the [`TypeId`] of the given variable, if it exists.
    pub fn get_variable_type(&self, ty: VariableId) -> Option<&TypeId> {
        self.types_module.get_variable(&ty)
    }
}
