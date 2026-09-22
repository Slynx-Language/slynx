use crate::{
    HirViewer,
    term::{Term, TermId},
};

impl HirViewer<'_, TermId> {
    pub fn raw(&self) -> &Term {
        self.hir.types.storage.terms.get(self.data)
    }
}
