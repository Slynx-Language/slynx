use crate::IRTypeId;

use super::{IRPointer, Label};

#[derive(Debug, Clone)]
///A context is anything that can be executed. It contains labels and each determine what to do, the '$entry' label is the label that is initally executed when this context initializes to be executed, for sure, this after compilation
pub struct Context {
    ///Named labels that can have instructions and determine what on the code to be executed. The first label this points to is the `$entry` label
    labels: IRPointer<Label>,
    ///The type of the context. Must be either Function
    ty: IRTypeId,
}

impl Context {
    pub fn new(ty: IRTypeId) -> Self {
        Self {
            labels: IRPointer::new(0, 0),
            ty,
        }
    }

    #[inline]
    ///Returns the pointer to the labels of this context.
    pub fn labels_ptr(&self) -> IRPointer<Label> {
        self.labels.clone()
    }

    ///Retrieves the inner type of this context
    #[inline]
    pub fn ty(&self) -> IRTypeId {
        self.ty
    }

    ///Sets the pointer part of the labels to the provided value.
    pub fn set_ptr(&mut self, ptr: usize) {
        self.labels.set_ptr(ptr);
    }

    ///Inserts a new label into the context, increasing the length of the labels pointer.
    pub fn insert_label(&mut self) {
        self.labels.increase_length();
    }

    ///Returns a pointer to the label at the given index
    pub fn get_label(&self, index: usize) -> IRPointer<Label, 1> {
        self.labels.ptr_to(index)
    }
}
