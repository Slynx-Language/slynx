use slynx_hir::{HirType, SlynxHir};
use slynx_ir::{IRTypeId, SlynxIR};

use crate::{Codegen, CodegenError, TypeId};

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
        let view = hir.view(*ty);
        let out = match view.dereference().raw() {
            HirType::Int => ir.int_type(),
            HirType::Float => ir.float_type(),
            HirType::Bool => ir.bool_type(),
            HirType::Void => ir.void_type(),
            HirType::Str => ir.str_type(),
            HirType::GenericComponent => ir.generic_component_type(),
            _ if let Some(mapped) = self.get_mapped_type(ty) => mapped,
            _ if let Some(viewer) = view.is_tuple() => {
                let ir_fields = {
                    let mut out = Vec::with_capacity(viewer.fields().len());
                    for field in viewer.fields() {
                        out.push(self.get_or_create_ir_type(field, hir, ir)?);
                    }
                    out
                };
                ir.create_or_get_tuple(ir_fields)
            }

            _ => return Err(CodegenError::IRTypeNotRecognized(*ty)),
        };
        Ok(out)
    }
}
