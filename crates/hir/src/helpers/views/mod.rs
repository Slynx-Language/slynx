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
impl<T> HirViewer<'_, T> {
    pub fn new_with<D>(&self, data: D) -> HirViewer<'_, D> {
        HirViewer {
            hir: self.hir,
            data,
        }
    }
}
