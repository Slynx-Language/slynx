mod enums;
mod structs;
use std::collections::HashMap;

use common::pool::{DedupPoolId, PoolId};
use slynx_hir::{HirExpression, HirType, SlynxHir};
use slynx_ir::{IRStructFlags, IRTypeId, SlynxIR};

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
    types: HashMap<DedupPoolId<HirType>, IRTypeId>,
}

impl<'a> TypeLowerer<'a> {
    pub fn new(hir: &'a SlynxHir<'a>) -> Self {
        Self {
            hir,
            enum_layouts: HashMap::new(),
            types: HashMap::new(),
        }
    }

    ///Generates a type name for use inside a Nullable struct name. Nested
    ///containers are encoded recursively so the produced name carries no
    ///special characters (e.g. `[4][]int` -> `ArrayVectorint4`).
    fn nullable_inner_name(&self, ty: &DedupPoolId<HirType>) -> String {
        let view = self.hir.view(*ty);
        if let Some(vec_inner) = view.is_vector() {
            format!("Vector{}", self.nullable_inner_name(&vec_inner))
        } else if let Some((arr_inner, len)) = view.is_array() {
            format!("Array{}{}", self.nullable_inner_name(&arr_inner), len)
        } else {
            view.name()
        }
    }

    ///Registers an already-known HIR→IR mapping (used by declaration hoisting,
    ///where the IR handle is created up-front and non-primitives resolve to it).
    pub(crate) fn register_mapping(&mut self, hir_ty: TypeId, ir_ty: IRTypeId) {
        self.types.insert(hir_ty, ir_ty);
    }

    ///The registered [`EnumLayout`] for an enum type, if it has been
    ///materialized by `insert_enum_fields_for`.
    pub(crate) fn enum_layout(&self, ty: &TypeId) -> Option<&EnumLayout> {
        self.enum_layouts.get(ty)
    }
    pub(crate) fn get_or_create_ir_type(
        &mut self,
        ty: TypeId,
        ir: &mut SlynxIR,
    ) -> Result<IRTypeId, CodegenError> {
        let view = self.hir.view(ty);
        let out = match view.dereference().raw() {
            HirType::Int => ir.int_type(),
            HirType::Float => ir.float_type(),
            HirType::Bool => ir.bool_type(),
            HirType::Void => ir.void_type(),
            HirType::Str => ir.str_type(),
            HirType::GenericComponent => ir.generic_component_type(),
            _ if let Some(mapped) = self.get_mapped_type(&ty) => mapped,
            _ if let Some(viewer) = view.is_tuple() => {
                let ir_fields = {
                    let mut out = Vec::with_capacity(viewer.fields().len());
                    for field in viewer.fields() {
                        out.push(self.get_or_create_ir_type(*field, ir)?);
                    }
                    out
                };
                ir.create_or_get_tuple(ir_fields)
            }
            HirType::Array(t, len) => {
                let ty = self.get_or_create_ir_type(*t, ir)?;
                ir.create_array(ty, *len)
            }
            HirType::Vector(t) => {
                let ty = self.get_or_create_ir_type(*t, ir)?;
                ir.create_vector(ty)
            }
            HirType::Nullable(inner) => {
                let name = self.nullable_inner_name(inner);
                let inner_type = self.get_or_create_ir_type(*inner, ir)?;
                let boolean = ir.bool_type();
                //struct {T, bool}
                ir.create_struct_full(
                    &format!("Nullable{name}"),
                    vec![inner_type, boolean],
                    IRStructFlags::NULLABLE,
                )
            }
            HirType::ImutableRef(t) | HirType::MutableRef(t) => {
                let ty = self.get_or_create_ir_type(*t, ir)?;
                ir.pointer_type(ty)
            }
            HirType::Enum(_) => {
                let key = view.dereference().data();
                // Layout materialization lives in one place:
                // `insert_enum_fields_for` registers the struct fields, the
                // payload union and the `EnumLayout` together (idempotently),
                // so this on-demand branch and the hoist pass agree on the
                // same shape every time.
                self.insert_enum_fields_for(key, ir)?;
                self.get_mapped_type(&key).ok_or_else(|| {
                    CodegenError::InternalError(
                        "enum layout was materialized but its struct is not mapped".into(),
                    )
                })?
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
        let struct_view = expr_view
            .ty_viewer()
            .concrete_type()
            .is_struct()
            .ok_or_else(|| {
                CodegenError::InternalError("Field access should be made on a struct type".into())
            })?;
        let field_type = struct_view.field_types()[field_index];
        let field_type = self.get_or_create_ir_type(field_type, ir)?;
        Ok(ir.pointer_type(field_type))
    }
}
