mod enums;
mod structs;
use std::collections::HashMap;

use common::pool::PoolId;
use slynx_hir::{
    DescriptorId, HirExpression, SlynxHir,
    term::{PrimitiveType, TermId, TermNode},
};
use slynx_ir::{IRTypeId, SlynxIR};

use crate::{CodegenError, TypeId};

///The IR layout of an enum type.
///
///An enum whose variants carry payloads is lowered to a struct whose `field[0]`
///holds the variant discriminant as an `int` tag and whose `field[1]` is a
///union: `struct {int tag, %{name}_payload}`. The union holds one member per
///variant (member index == variant index), where each member is that variant's
///payload struct (`%{name}_variant_{Variant}`). Enums whose variants carry no
///payload at all lower to a bare `struct {int tag}`.
///
///This is the single source of truth for the enum layout: `insert_enum_fields_for`
///materializes the struct fields and registers the layout, while both enum
///construction (`lower_enum`) and pattern matching (`lower_matches`) consume it
///instead of re-deriving the shape from the IR.
#[derive(Clone)]
pub struct EnumLayout {
    ///The IR struct type of the enum.
    pub type_id: IRTypeId,
    ///The payload struct id for each variant (member index == variant index).
    ///`None` only when a variant has no payload, although an empty payload
    ///struct is still emitted and stored so indices stay aligned.
    pub variant_payload: Vec<Option<IRTypeId>>,
    ///The union type holding every variant's payload struct. `Some` only when
    ///at least one variant carries a payload.
    pub union_type: Option<IRTypeId>,
}

pub struct TypeLowerer<'a> {
    hir: &'a SlynxHir<'a>,
    /// IR layouts for enum types.
    enum_layouts: HashMap<TypeId, EnumLayout>,
    types: HashMap<TermId, IRTypeId>,
}

impl<'a> TypeLowerer<'a> {
    pub fn new(hir: &'a SlynxHir<'a>) -> Self {
        Self {
            hir,
            enum_layouts: HashMap::new(),
            types: HashMap::new(),
        }
    }

    ///Registers an already-known HIR→IR mapping (used by declaration hoisting,
    ///where the IR handle is created up-front and non-primitives resolve to it).
    pub(crate) fn register_mapping(&mut self, hir_ty: TypeId, ir_ty: IRTypeId) {
        self.types.insert(hir_ty, ir_ty);
    }

    ///The registered [`EnumLayout`] for an enum type. Returns a single typed
    ///error rather than letting every consumer construct a near-identical
    ///"layout is not registered" string.
    pub(crate) fn enum_layout(&self, ty: &TypeId) -> Result<&EnumLayout, CodegenError> {
        self.enum_layouts
            .get(ty)
            .ok_or(CodegenError::MissingEnumLayout(*ty))
    }
    pub(crate) fn get_or_create_ir_type(
        &mut self,
        ty: TypeId,
        ir: &mut SlynxIR,
    ) -> Result<IRTypeId, CodegenError> {
        // `dereference` unrolls named-type references (the term form of the
        // old `HirType::Reference`) while leaving arrays, vectors and raw
        // `Ref`s intact, mirroring the classic `get_or_create_ir_type` that
        // matched on `view.dereference().raw()`.
        let deref = self.hir.view(ty).dereference();
        let out = match deref.raw().node() {
            TermNode::Primitive(PrimitiveType::Signed { .. }) => ir.types.int_type(),
            TermNode::Primitive(PrimitiveType::Unsigned { bitsize: 1 }) => ir.types.bool_type(),
            TermNode::Primitive(PrimitiveType::Unsigned { .. }) => ir.types.int_type(),
            TermNode::Primitive(PrimitiveType::Float32 | PrimitiveType::Float64) => {
                ir.types.float_type()
            }
            TermNode::Primitive(PrimitiveType::Void) => ir.types.void_type(),
            TermNode::Primitive(PrimitiveType::String) => ir.types.str_type(),
            TermNode::Extension(_) => ir.types.generic_component_type(),
            _ if let Some(mapped) = self.get_mapped_type(&ty) => mapped,
            _ if let Some(fields) = deref.is_tuple() => {
                let ir_fields = {
                    let mut out = Vec::with_capacity(fields.len());
                    for field in fields {
                        out.push(self.get_or_create_ir_type(*field, ir)?);
                    }
                    out
                };
                ir.types.create_or_get_tuple(ir_fields)
            }
            _ if let Some((elem, len)) = deref.is_array() => {
                let elem_ty = self.get_or_create_ir_type(elem, ir)?;
                ir.create_array(elem_ty, len)
            }
            _ if let Some(elem) = deref.is_vector() => {
                let elem_ty = self.get_or_create_ir_type(elem, ir)?;
                ir.create_vector(elem_ty)
            }
            _ if let Some(target) = deref.is_imutable_ref().or_else(|| deref.is_mutable_ref()) => {
                let target_ty = self.get_or_create_ir_type(target, ir)?;
                ir.types.pointer_type(target_ty)
            }
            TermNode::Data(DescriptorId::Enum(_)) => {
                let key = deref.data();
                // Layout materialization lives in one place:
                // `insert_enum_fields_for` registers the struct fields, the
                // payload union and the `EnumLayout` together (idempotently),
                // so this on-demand branch and the hoist pass agree on the
                // same shape every time.
                self.insert_enum_fields_for(key, ir)?
            }

            _ => return Err(CodegenError::IRTypeNotRecognized(ty)),
        };
        Ok(out)
    }

    pub(crate) fn get_mapped_type(&self, ty: &TypeId) -> Option<IRTypeId> {
        self.types.get(ty).cloned()
    }

    ///Computes the IR pointer type of the struct field at `field_index`, for a
    ///field access reached through a deref of `inner`. Both the field-access
    ///read and write paths share this so the struct-reference check and the
    ///pointer-type computation happen in one place.
    pub(crate) fn deref_field_type(
        &mut self,
        inner: PoolId<HirExpression>,
        field_index: usize,
        ir: &mut SlynxIR,
    ) -> Result<IRTypeId, CodegenError> {
        let expr_view = self.hir.view(inner);
        let concrete = expr_view.ty_viewer().concrete_type();
        let concrete_ty = concrete.data();
        let struct_view = concrete
            .is_struct()
            .ok_or(CodegenError::NotAStruct(concrete_ty))?;
        let field_type = struct_view.field_types()[field_index];
        let field_type = self.get_or_create_ir_type(field_type, ir)?;
        Ok(ir.types.pointer_type(field_type))
    }
}
