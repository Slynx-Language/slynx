use common::pool::DedupPoolId;
use slynx_hir::{HirType, SlynxHir};
use slynx_ir::{IRStructFlags, IRTypeId, SlynxIR};

use crate::{Codegen, CodegenError, TypeId};

impl Codegen {
    ///Generates a type name for use inside a Nullable struct name. Nested
    ///containers are encoded recursively so the produced name carries no
    ///special characters (e.g. `[4][]int` -> `ArrayVectorint4`).
    fn nullable_inner_name(&self, ty: &DedupPoolId<HirType>, hir: &SlynxHir) -> String {
        let view = hir.view(*ty);
        if let Some(vec_inner) = view.is_vector() {
            format!("Vector{}", self.nullable_inner_name(&vec_inner, hir))
        } else if let Some((arr_inner, len)) = view.is_array() {
            format!("Array{}{}", self.nullable_inner_name(&arr_inner, hir), len)
        } else {
            view.name()
        }
    }

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
                let name = self.nullable_inner_name(inner, hir);
                let inner_type = self.get_or_create_ir_type(inner, hir, ir)?;
                let boolean = ir.bool_type();
                //struct {T, bool}
                ir.create_struct_full(
                    &format!("Nullable{name}"),
                    vec![inner_type, boolean],
                    IRStructFlags::NULLABLE,
                )
            }
            HirType::ImutableRef(t) | HirType::MutableRef(t) => {
                let ty = self.get_or_create_ir_type(t, hir, ir)?;
                ir.pointer_type(ty)
            }

            _ => return Err(CodegenError::IRTypeNotRecognized(*ty)),
        };
        Ok(out)
    }
}
