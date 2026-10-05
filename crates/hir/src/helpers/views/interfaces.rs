use common::pool::DedupPoolId;

use crate::{HirViewer, InterfaceType};

impl HirViewer<'_, DedupPoolId<InterfaceType>> {
    pub fn raw(&self) -> &InterfaceType {
        self.hir.types.storage.interfaces.get(self.data)
    }
}
