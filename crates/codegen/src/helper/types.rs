use common::pool::DedupPoolId;
use slynx_hir::{HirType, SlynxHir};
use slynx_ir::IRType;

use crate::{Codegen, CodegenError};

impl Codegen {
    pub(crate) fn insert_object_fields_for(
        &mut self,
        decl: DedupPoolId<HirType>,
        hir: &SlynxHir,
        ir: &mut slynx_ir::SlynxIR,
    ) -> Result<(), CodegenError> {
        let obj_handle = self
            .get_mapped_type(&decl)
            .ok_or(CodegenError::IRTypeNotRecognized(decl))?;
        let IRType::Struct(obj) = ir.get_type(obj_handle) else {
            unreachable!();
        };
        let fields = if let Some(viewer) = hir.view(decl).dereference().is_struct() {
            viewer.field_types().to_vec()
        } else {
            unreachable!("{:?} should map to an Object, but it doesn't", decl)
        };

        for field in &fields {
            let ty = self.get_or_create_ir_type(field, hir, ir)?;
            let obj_ty = ir.get_object_type_mut(obj);
            obj_ty.insert_field(ty);
        }
        Ok(())
    }
}
