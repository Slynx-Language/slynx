use common::pool::DedupPoolId;

use crate::{HirType, StyleType, helpers::HirViewer};

impl HirViewer<'_, DedupPoolId<StyleType>> {
    pub fn args(&self) -> &[DedupPoolId<HirType>] {
        &self.hir.types_module[self.data].args
    }
}
