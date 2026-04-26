use crate::hir::{SlynxHir, TypeId, model::HirType};

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
}
