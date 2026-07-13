use common::pool::DedupPoolId;

use crate::{SlynxHir, VariableId, model::HirType};

impl<'a> SlynxHir<'a> {
    /// Returns the [`DedupPoolId<HirType>`] of the given variable, if it exists.
    pub fn get_variable_type(&self, ty: VariableId) -> Option<DedupPoolId<HirType>> {
        self.types_module.get_variable(&ty)
    }
}
