use crate::{SlynxHir, VariableId, term::TermId};

impl<'a> SlynxHir<'a> {
    /// Returns the [`TermId`] of the given variable, if it exists.
    pub fn get_variable_type(&self, ty: VariableId) -> Option<TermId> {
        self.types.get_variable(&ty)
    }
}
