mod storage;
pub use storage::*;
mod component;
mod function;
mod instructions;

use crate::{IRPointer, SlynxIR};

/// A single-element view over an IR storage array.
pub struct IRViewer<'a, T> {
    pub(crate) ptr: IRPointer<T, 1>,
    pub(crate) ir: &'a SlynxIR,
}

impl<'a, T> IRViewer<'a, T>
where
    SlynxIR: IRStorage<T>,
{
    pub fn value(&self) -> &T {
        self.ir.get_idx(self.ptr.ptr())
    }
}

/// A view over a range of elements in an IR storage array.
pub struct IRBatchViewer<'a, T> {
    pub(crate) ptr: IRPointer<T>,
    pub(crate) ir: &'a SlynxIR,
}

impl<'a, T> IRBatchViewer<'a, T> {
    pub fn at(&self, index: usize) -> IRViewer<'a, T> {
        IRViewer {
            ptr: self.ptr.ptr_to(index),
            ir: self.ir,
        }
    }
    pub fn values(&self) -> &[T]
    where
        SlynxIR: IRStorage<T>,
    {
        let range = self.ptr.range();
        &self.ir.get_storage()[range]
    }
}
