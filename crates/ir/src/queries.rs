use crate::{IRBatchViewer, IRPointer, IRViewer, SlynxIR};

impl SlynxIR {
    pub fn get_view<'a, T>(&'a self, ptr: IRPointer<T, 1>) -> IRViewer<'a, T> {
        IRViewer { ptr, ir: self }
    }

    pub fn get_batch_view<'a, T>(&'a self, ptr: IRPointer<T>) -> IRBatchViewer<'a, T> {
        IRBatchViewer { ptr, ir: self }
    }
}
