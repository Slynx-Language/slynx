use crate::SlynxHir;
mod component;
mod declarations;
mod expressions;
mod functions;
mod strukt;
mod styles;
mod types;

pub struct HirViewer<'a, T> {
    pub(crate) hir: &'a SlynxHir<'a>,
    pub(crate) data: T,
}
impl<'a, T> HirViewer<'a, T> {
    pub fn new_with<D>(self, data: D) -> HirViewer<'a, D> {
        HirViewer {
            hir: self.hir,
            data,
        }
    }
}

impl<T: Copy> HirViewer<'_, T> {
    ///The underlying id this viewer wraps. Pool ids are `Copy`, so returning
    ///the id by value lets callers outside the HIR crate (e.g. codegen) key
    ///caches off e.g. the raw enum type id of a dereferenced reference.
    pub fn data(&self) -> T {
        self.data
    }
}

impl<T: Clone> Clone for HirViewer<'_, T> {
    fn clone(&self) -> Self {
        Self {
            hir: self.hir,
            data: self.data.clone(),
        }
    }
}
impl<T> Copy for HirViewer<'_, T> where T: Copy {}
