use slynx_hir::{HirType, SlynxHir, TypeId};
use slynx_ir::{IRTypeId, SlynxIR};

use crate::{Codegen, CodegenError};

impl Codegen {
    pub(crate) fn get_mapped_type(&self, ty: &TypeId) -> Option<IRTypeId> {
        self.types.get(ty).cloned()
    }

    pub(crate) fn get_or_create_ir_type(
        &self,
        ty: &TypeId,
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<IRTypeId, CodegenError> {
        let ty = *ty;
        let out = match ty {
            _ if ty == hir.int32_type() => ir.int_type(),
            _ if ty == hir.float32_type() => ir.float_type(),
            _ if ty == hir.bool_type() => ir.bool_type(),
            _ if ty == hir.void_type() => ir.void_type(),
            _ if ty == hir.str_type() => ir.str_type(),
            _ if ty == hir.component_type() => ir.generic_component_type(),
            _ if let Some(mapped) = self.get_mapped_type(&ty) => mapped,
            _ if let HirType::Tuple { fields } = hir.get_type(&ty) => {
                let ir_fields = {
                    let mut out = Vec::with_capacity(fields.len());
                    for field in fields.clone() {
                        out.push(self.get_or_create_ir_type(&field, hir, ir)?);
                    }
                    out
                };
                ir.create_or_get_tuple(ir_fields)
            }
            _ if let HirType::Reference { rf, .. } = hir.get_type(&ty) => {
                let rf = *rf;
                return self.get_or_create_ir_type(&rf, hir, ir);
            }
            _ => return Err(CodegenError::IRTypeNotRecognized(ty)),
        };
        Ok(out)
    }
}
