use slynx_hir::{VariableId, id::AnyDeclarationId};

use crate::TypeId;

#[derive(Debug)]
///An error that occurred on the IR
pub enum CodegenError {
    ///The provided type from the HIR was not recognized on the IR
    IRTypeNotRecognized(TypeId),
    DeclarationNotRecognized(AnyDeclarationId),
    UnrecognizedVariable(VariableId),
    InternalError(String),
}
