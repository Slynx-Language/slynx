use common::pool::{DedupPoolId, PoolId};

use crate::{HirExpression, HirType, helpers::HirViewer};

impl HirViewer<'_, PoolId<HirExpression>> {
    pub fn ty(&self) -> DedupPoolId<HirType> {
        self.hir[self.data].ty
    }
    pub fn ty_viewer(&self) -> HirViewer<'_, DedupPoolId<HirType>> {
        self.new_with(self.ty())
    }
}
