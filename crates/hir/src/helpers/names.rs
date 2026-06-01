use common::Span;

use crate::{HIRError, Result, SlynxHir, SymbolPointer, VariableId};

impl SlynxHir {
    ///Tries to retrieve a variable with the provided `name` on the current active scope
    pub fn get_variable(&mut self, symbol: SymbolPointer, span: &Span) -> Result<VariableId> {
        if let Some(variable) = self.modules.find_variable(symbol) {
            Ok(variable)
        } else {
            Err(HIRError::name_unrecognized(symbol, *span))
        }
    }
    ///Retrieves the pointer(simply a symbol) of the provided `name`.
    pub fn get_symbol(&self, name: &str) -> Option<SymbolPointer> {
        self.modules.retrieve_symbol(name)
    }
}
