use crate::{Component, Function, Instruction, IRPointer, Label, SlynxIR};

/// Trait providing indexed access to flat storage arrays inside [`SlynxIR`].
pub trait IRStorage<T> {
    fn get_storage(&self) -> &[T];
    fn get_storage_mut(&mut self) -> &mut [T];

    /// Access element at a raw index.
    fn get_idx(&self, idx: usize) -> &T {
        &self.get_storage()[idx]
    }
    fn get_idx_mut(&mut self, idx: usize) -> &mut T {
        &mut self.get_storage_mut()[idx]
    }

    /// Access element by [`IRPointer`].
    fn get(&self, ptr: IRPointer<T, 1>) -> &T {
        self.get_idx(ptr.ptr())
    }
    fn get_mut(&mut self, ptr: IRPointer<T, 1>) -> &mut T {
        self.get_idx_mut(ptr.ptr())
    }
}

macro_rules! impl_storage {
    ($ty:ty, $field:ident) => {
        impl IRStorage<$ty> for SlynxIR {
            fn get_storage(&self) -> &[$ty] {
                &self.$field
            }
            fn get_storage_mut(&mut self) -> &mut [$ty] {
                &mut self.$field
            }
        }
    };
}

impl_storage!(Function, functions);
impl_storage!(Label, labels);
impl_storage!(Component, components);
impl_storage!(Instruction, instructions);
