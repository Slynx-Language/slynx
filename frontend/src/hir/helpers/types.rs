use crate::hir::{SlynxHir, TypeId};

impl SlynxHir {
    pub fn infer_type(&self) -> TypeId {
        self.modules.types_module.infer_id()
    }
    pub fn int32_type(&self) -> TypeId {
        self.modules.types_module.int_id()
    }
    pub fn float32_type(&self) -> TypeId {
        self.modules.types_module.float_id()
    }
    pub fn void_type(&self) -> TypeId {
        self.modules.types_module.void_id()
    }
    pub fn bool_type(&self) -> TypeId {
        self.modules.types_module.bool_id()
    }
    pub fn str_type(&self) -> TypeId {
        self.modules.types_module.str_id()
    }
    pub fn component_type(&self) -> TypeId {
        self.modules.types_module.generic_component_id()
    }
}
