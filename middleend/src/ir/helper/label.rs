use crate::{SlynxIR, ir::model::{IRPointer, Label}};

impl SlynxIR {
    /// Returns a reference to the label at the given pointer.
    pub fn get_label(&self, label: IRPointer<Label>) -> &Label {
        &self.labels[label.ptr()]
    }
    
    /// Returns a mutable reference to the label at the given pointer.
    pub fn get_label_mut(&mut self, label: IRPointer<Label>) -> &mut Label {
        &mut self.labels[label.ptr()]
    }
}