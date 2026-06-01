use crate::{IRError, IRErrorDescription, IRErrorKind, IRStructId, IRType, IRTypeId, SlynxIR};

pub struct StructBuilder<'a> {
    ir: &'a mut SlynxIR,
    ir_struct_id: IRStructId,
    ty: IRTypeId,
}

impl<'a> StructBuilder<'a> {
    ///Creates a new Struct builder with the given `ty` and inner `ir` to build properly. Returns error if the given `ty` is not a struct type
    pub fn new(ty: IRTypeId, ir: &'a mut SlynxIR) -> Result<Self, IRError> {
        let IRType::Struct(struct_id) = ir.types.get_type(ty) else {
            return Err(IRError::new(
                IRErrorKind::InvalidType,
                IRErrorDescription::NotAStruct,
            ));
        };

        Ok(Self {
            ir,
            ty,
            ir_struct_id: struct_id,
        })
    }
    pub fn insert_type(&mut self, ty: IRTypeId) {
        self.ir
            .types
            .get_object_type_mut(self.ir_struct_id)
            .insert_field(ty);
    }

    pub fn get_field(&self, index: usize) -> IRTypeId {
        self.ir
            .types
            .get_object_type(self.ir_struct_id)
            .get_fields()[index]
    }

    pub fn generate(self) -> IRTypeId {
        self.ty
    }
}
