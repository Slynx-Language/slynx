use slynx_hir::{VariableId, id::AnyDeclarationId};
use slynx_ir::IRError;

use crate::TypeId;

#[derive(Debug)]
///An error that occurred on the IR
pub enum CodegenError {
    ///The provided type from the HIR was not recognized on the IR
    IRTypeNotRecognized(TypeId),
    DeclarationNotRecognized(AnyDeclarationId),
    UnrecognizedVariable(VariableId),
    ///The provided type is not an enum, but enum-shaped lowering was requested.
    NotAnEnum(TypeId),
    ///The provided type is not a struct, but struct-shaped lowering was requested.
    NotAStruct(TypeId),
    ///The variant index is out of bounds for the enum's variant list.
    InvalidVariantIndex(TypeId, usize),
    ///The enum type's layout was never materialized, so construction or
    ///pattern matching cannot produce IR that depends on its shape.
    MissingEnumLayout(TypeId),
    ///The enum's layout is missing a payload piece the HIR lowering relies on
    ///(the per-variant payload struct or the payload union).
    MissingEnumPayload(TypeId),
    ///An error raised by the IR builder while lowering a function body.
    IRError(IRError),
}

impl From<IRError> for CodegenError {
    fn from(error: IRError) -> Self {
        CodegenError::IRError(error)
    }
}
