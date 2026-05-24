mod storage;
pub use storage::*;
mod component;
mod function;
mod instructions;
mod value;

use crate::{IRPointer, SlynxIR};

pub struct IRViewer<'a, T> {
    pub(crate) ptr: IRPointer<T, 1>,
    pub(crate) ir: &'a SlynxIR,
}

impl<'a, T> IRViewer<'a, T>
where
    SlynxIR: IRStorage<T>,
{
    pub fn value(&self) -> &T {
        self.ir.get(self.ptr)
    }
}
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
        &self.ir.get_storage()[self.ptr.range()]
    }
}

impl<'a, T> IRBatchViewer<'a, IRPointer<T>> {
    pub fn flattened_values(&self) -> Vec<&T>
    where
        SlynxIR: IRStorage<IRPointer<T>> + IRStorage<T>,
    {
        let ptrs = self.values();
        ptrs.iter()
            .flat_map(|ptr| self.ir.get_storage()[ptr.range()].iter())
            .collect()
    }
}
