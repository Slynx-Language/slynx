use slynx_hir::term::TermId;
use slynx_ir::{IRType, IRTypeId};

use crate::{
    CodegenError,
    lowerers::{EnumLayout, TypeLowerer},
};

impl<'a> TypeLowerer<'a> {
    ///Materializes the IR layout for a hoisted enum: a struct whose `field[0]`
    ///holds the variant discriminant tag and whose `field[1]` is a union of the
    ///per-variant payload structs (member index == variant index). Enums whose
    ///variants carry no payload lower to a bare `struct {int tag}`.
    ///
    ///Idempotent: if the enum struct already has fields the layout is left
    ///untouched. Registers the resulting [`EnumLayout`] so construction and
    ///pattern matching can resolve the union and per-variant payload structs.
    ///Materializes the IR layout for an enum type and registers it.
    ///
    ///This is the single source of truth for the enum layout contract: an enum
    ///lowers to a struct whose `field[0]` is the `int` discriminant tag and
    ///whose optional `field[1]` is a union of the per-variant payload structs
    ///(`%{name}_variant_{Variant}`), union member index == variant index.
    ///Both construction (`lower_enum`) and pattern matching (`lower_matches`)
    ///consume the registered [`EnumLayout`] instead of re-deriving the shape
    ///from the IR. It is idempotent: once the layout is registered (or the
    ///struct already carries fields) it returns immediately, so both the hoist
    ///pass and the on-demand path in `get_or_create_ir_type` can share it.
    pub(crate) fn insert_enum_fields_for(
        &mut self,
        decl: TermId,
        ir: &mut slynx_ir::SlynxIR,
    ) -> Result<IRTypeId, CodegenError> {
        let key = self.hir.view(decl).dereference().data();
        if self.enum_layouts.contains_key(&key) {
            // The layout and the struct mapping are registered together, so a
            // registered layout implies the struct is already mapped.
            return self
                .get_mapped_type(&key)
                .ok_or(CodegenError::MissingEnumLayout(key));
        }
        let enum_struct = match self.get_mapped_type(&key) {
            Some(ty) => ty,
            None => {
                let enum_view = self
                    .hir
                    .view(key)
                    .dereference()
                    .is_enum()
                    .ok_or(CodegenError::NotAnEnum(key))?;
                let name = self.hir.get_name(enum_view.name());
                let ty = ir.create_struct(name);
                self.types.insert(key, ty);
                ty
            }
        };
        let IRType::Struct(enum_struct_id) = *ir.types.get_type(enum_struct) else {
            return Err(CodegenError::NotAStruct(key));
        };
        if !ir
            .types
            .get_object_type(enum_struct_id)
            .get_fields()
            .is_empty()
        {
            return Ok(enum_struct);
        }
        let enum_view = self
            .hir
            .view(decl)
            .dereference()
            .is_enum()
            .ok_or(CodegenError::NotAnEnum(key))?;
        let enum_name = self.hir.get_name(enum_view.name());
        let int_type = ir.types.int_type();

        let has_payload = enum_view
            .variants()
            .iter()
            .any(|variant| !variant.payload.is_empty());

        let mut variant_payload = Vec::with_capacity(enum_view.variants().len());
        let mut union_type = None;

        if has_payload {
            let union_name = format!("{enum_name}_payload");
            let union_ty = ir.create_union(&union_name);
            let IRType::Union(union_id) = *ir.types.get_type(union_ty) else {
                return Err(CodegenError::MissingEnumPayload(key));
            };
            let mut members = Vec::with_capacity(enum_view.variants().len());
            for variant in enum_view.variants() {
                let payload_struct_name =
                    format!("{enum_name}_variant_{}", self.hir.get_name(variant.name));
                let payload_struct = ir.create_struct(&payload_struct_name);
                let IRType::Struct(payload_struct_id) = *ir.types.get_type(payload_struct) else {
                    unreachable!("create_struct must produce a struct");
                };
                for payload_ty in &variant.payload {
                    let field_ty = self.get_or_create_ir_type(*payload_ty, ir)?;
                    ir.types
                        .get_object_type_mut(payload_struct_id)
                        .insert_field(field_ty);
                }
                members.push(payload_struct);
                variant_payload.push(Some(payload_struct));
            }
            {
                let union = ir.types.get_union_type_mut(union_id);
                for member in members {
                    union.insert_variant(member);
                }
            }
            union_type = Some(union_ty);

            let struct_ir = ir.types.get_object_type_mut(enum_struct_id);
            struct_ir.insert_field(int_type);
            struct_ir.insert_field(union_ty);
        } else {
            variant_payload = enum_view.variants().iter().map(|_| None).collect();
            ir.types
                .get_object_type_mut(enum_struct_id)
                .insert_field(int_type);
        }

        self.enum_layouts.insert(
            key,
            EnumLayout {
                type_id: enum_struct,
                variant_payload,
                union_type,
            },
        );
        Ok(enum_struct)
    }
}
