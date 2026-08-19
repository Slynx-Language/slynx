use common::pool::DedupPoolId;

use crate::{HirType, StyleType, helpers::HirViewer};

impl HirViewer<'_, DedupPoolId<StyleType>> {
    pub fn args(&self) -> &[DedupPoolId<HirType>] {
        &self.hir.types_module[self.data].args
    }
    pub fn name(&self) -> String {
        let name = self.hir.get_style_name(self.data);
        self.hir.get_name(name).to_string()
    }
}
