use std::backtrace::Backtrace;

use slynx_hir::{VariableId, id::AnyDeclarationId, term::TermId};
use slynx_ir::IRError;

#[derive(Debug)]
pub struct CodegenError {
    pub kind: CodegenErrorKind,
    pub backtrace: Backtrace,
}

impl CodegenError {
    pub fn new(kind: CodegenErrorKind) -> Self {
        Self {
            kind,
            backtrace: Backtrace::capture(),
        }
    }
}

#[derive(Debug)]
///An error that occurred on the IR
pub enum CodegenErrorKind {
    ///The provided type from the HIR was not recognized on the IR
    IRTypeNotRecognized(TermId),
    DeclarationNotRecognized(AnyDeclarationId),
    UnrecognizedVariable(VariableId),
    ///The provided type is not an enum, but enum-shaped lowering was requested.
    NotAnEnum(TermId),
    ///The provided type is not a struct, but struct-shaped lowering was requested.
    NotAStruct(TermId),
    ///The variant index is out of bounds for the enum's variant list.
    InvalidVariantIndex(TermId, usize),
    ///The enum type's layout was never materialized, so construction or
    ///pattern matching cannot produce IR that depends on its shape.
    MissingEnumLayout(TermId),
    ///The enum's layout is missing a payload piece the HIR lowering relies on
    ///(the per-variant payload struct or the payload union).
    MissingEnumPayload(TermId),
    ///An error raised by the IR builder while lowering a function body.
    IRError(IRError),
}

impl From<IRError> for CodegenError {
    fn from(error: IRError) -> Self {
        CodegenError::new(CodegenErrorKind::IRError(error))
    }
}
