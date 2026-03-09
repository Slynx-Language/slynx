use crate::{IRPointer, IRTypeId, ir::label::Label};

///A context is anything that can be executed. It contains labels and each determine what to do, the '$entry' label is the label that is initally executed when this context initializes to be executed, for sure, this after compilation
pub struct Context {
    ///Named labels that can have instructions and determine what on the code to be executed. The first label this points to is the `$entry` label
    labels: IRPointer<Label>,
    ///The type of the context. Must be either Function
    ty: IRTypeId,
}
