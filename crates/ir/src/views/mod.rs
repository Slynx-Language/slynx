mod component;
mod function;
mod instructions;
mod value;

use crate::{Component, Function, IRPointer, Instruction, Label, Operand, SlynxIR, Value};

pub trait IRStorage<T> {
    fn get_storage(&self) -> &[T];
    fn get_storage_mut(&mut self) -> &mut [T];
    fn get(&self, ptr: IRPointer<T, 1>) -> &T {
        &self.get_storage()[ptr.ptr()]
    }
    fn get_mut(&mut self, ptr: IRPointer<T, 1>) -> &mut T {
        &mut self.get_storage_mut()[ptr.ptr()]
    }
    fn get_ranged(&self, ptr: IRPointer<T, 0>) -> &[T] {
        &self.get_storage()[ptr.range()]
    }
    fn get_ranged_mut(&mut self, ptr: IRPointer<T, 0>) -> &mut [T] {
        &mut self.get_storage_mut()[ptr.range()]
    }
}

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
}

macro_rules! impl_storage {
    ($storage_type: ty, $storage_name: ident) => {
        paste::paste! {
            impl<'a> IRStorage<$storage_type> for SlynxIR {
                fn get_storage(&self) -> &[$storage_type] {
                    &self.$storage_name
                }
                fn get_storage_mut(&mut self) -> &mut [$storage_type] {
                    &mut self.$storage_name
                }
            }
        }
    };
}

impl_storage!(Function, functions);
impl_storage!(Label, labels);
impl_storage!(Component, components);
impl_storage!(Instruction, instructions);
impl_storage!(Value, values);
impl_storage!(Operand, operands);
