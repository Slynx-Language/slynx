use slynx_hir::{DeclarationId, TypeId, VariableId};

#[derive(Debug)]
///An error that occurred on the IR
pub enum IRError {
    ///The provided type from the HIR was not recognized on the IR
    IRTypeNotRecognized(TypeId),
    DeclarationNotRecognized(DeclarationId),
    UnrecognizedVariable(VariableId),
}
