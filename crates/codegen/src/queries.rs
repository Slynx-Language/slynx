use slynx_hir::{HirType, SlynxHir};
use slynx_ir::{IRType, IRTypeId, SlynxIR};

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
            HirType::Array(t, len) => {
                let ty = self.get_or_create_ir_type(t, hir, ir)?;
                ir.create_array(ty, *len)
            }
            HirType::Vector(t) => {
                let ty = self.get_or_create_ir_type(t, hir, ir)?;
                ir.create_vector(ty)
            }
            HirType::Nullable(inner) => {
                let name = hir.view(*inner).name();
                let s = ir.create_struct(&format!("Nullable{name}"));
                let inner_type = self.get_or_create_ir_type(&inner, hir, ir)?;
                let boolean = ir.bool_type();
                let IRType::Struct(strukt_id) = *ir.get_type(s) else {
                    unreachable!("Expected type from ir.create_struct to be a struct one");
                };
                let strukt = ir.get_object_type_mut(strukt_id);
                //struct {T, bool}
                strukt.insert_field(inner_type);
                strukt.insert_field(boolean);
                s
            }

            _ => return Err(CodegenError::IRTypeNotRecognized(*ty)),
        };
        Ok(out)
    }
}
