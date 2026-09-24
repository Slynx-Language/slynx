use slynx_hir::term::TermId;
use slynx_ir::IRType;

use crate::{CodegenError, lowerers::TypeLowerer};

impl<'a> TypeLowerer<'a> {
    pub(crate) fn insert_object_fields_for(
        &mut self,
        decl: TermId,
        ir: &mut slynx_ir::SlynxIR,
    ) -> Result<(), CodegenError> {
        let obj_handle = self
            .get_mapped_type(&decl)
            .ok_or(CodegenError::IRTypeNotRecognized(decl))?;
        let IRType::Struct(obj) = *ir.types.get_type(obj_handle) else {
            return Err(CodegenError::NotAStruct(decl));
        };
        let fields = if let Some(viewer) = self.hir.view(decl).dereference().is_struct() {
            viewer.field_types().to_vec()
        } else {
            return Err(CodegenError::NotAStruct(decl));
        };

        for field in &fields {
            let ty = self.get_or_create_ir_type(*field, ir)?;
            let obj_ty = ir.types.get_object_type_mut(obj);
            obj_ty.insert_field(ty);
        }
        Ok(())
    }
}
