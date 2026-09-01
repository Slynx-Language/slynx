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
        let IRType::Struct(obj) = *ir.get_type(obj_handle) else {
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

    ///Fills in the fields of an enum's IR struct: an `int` tag at `field[0]`
    ///followed by the payload fields of every variant, in declaration order.
    pub(crate) fn insert_enum_fields_for(
        &mut self,
        decl: DedupPoolId<HirType>,
        hir: &SlynxHir,
        ir: &mut slynx_ir::SlynxIR,
    ) -> Result<(), CodegenError> {
        let layout = self
            .enum_layouts
            .get(&decl)
            .ok_or(CodegenError::IRTypeNotRecognized(decl))?
            .type_id;
        let IRType::Struct(struct_id) = *ir.get_type(layout) else {
            unreachable!();
        };
        let fields = if let Some(viewer) = hir.view(decl).dereference().is_enum() {
            viewer
                .variants()
                .iter()
                .flat_map(|variant| variant.payload.iter().copied())
                .collect::<Vec<_>>()
        } else {
            unreachable!("{:?} should map to an Enum, but it doesn't", decl)
        };

        let tag_type = ir.int_type();
        let ir_type = ir.get_object_type_mut(struct_id);
        ir_type.insert_field(tag_type);
        for field in &fields {
            let ty = self.get_or_create_ir_type(field, hir, ir)?;
            let obj_ty = ir.get_object_type_mut(struct_id);
            obj_ty.insert_field(ty);
        }
        Ok(())
    }
}
