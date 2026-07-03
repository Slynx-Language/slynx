use common::pool::DedupPoolId;

use crate::{FunctionType, HirType, helpers::HirViewer};

impl HirViewer<'_, DedupPoolId<FunctionType>> {
    pub fn arguments(&self) -> &[DedupPoolId<HirType>] {
        &self.hir.types_module[self.data].args
    }
    pub fn return_type(&self) -> DedupPoolId<HirType> {
        self.hir.types_module[self.data].ret
    }
}
