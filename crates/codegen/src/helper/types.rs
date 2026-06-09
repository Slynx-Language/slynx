use slynx_hir::SlynxHir;
use slynx_ir::IRType;

use crate::{Codegen, CodegenError};

impl Codegen {
    pub(crate) fn insert_object_fields_for(
        &mut self,
        decl: slynx_hir::TypeId,
        hir: &SlynxHir,
        ir: &mut slynx_ir::SlynxIR,
    ) -> Result<(), CodegenError> {
        let obj_handle = self
            .get_mapped_type(&decl)
            .ok_or(CodegenError::IRTypeNotRecognized(decl))?;
        let IRType::Struct(obj) = ir.get_type(obj_handle) else {
            unreachable!();
        };
        let mut current = decl;
        let fields = loop {
            match &*hir.get_type(&current) {
                slynx_hir::HirType::Struct { fields } => break fields.clone(),
                slynx_hir::HirType::Reference { rf, .. } => current = *rf,
                _ => unreachable!("{:?} should map to an Object, but it doesn't", decl),
            }
        };
        for field in &fields {
            let ty = self.get_or_create_ir_type(field, hir, ir)?;
            let obj_ty = ir.get_object_type_mut(obj);
            obj_ty.insert_field(ty);
        }
        Ok(())
    }
}
