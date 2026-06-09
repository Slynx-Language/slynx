use common::Span;

use crate::{HIRError, Result, SlynxHir, SymbolPointer, VariableId, module_loader::FileId};

impl SlynxHir {
    ///Tries to retrieve a variable with the provided `name` on the current active scope
    pub fn get_variable(
        &self,
        fileid: FileId,
        symbol: SymbolPointer,
        span: &Span,
    ) -> Result<VariableId> {
        if let Some(variable) = self.get_file(fileid).scopes.get_name(&symbol) {
            Ok(variable.clone())
        } else {
            Err(HIRError::name_unrecognized(symbol, *span))
        }
    }
    ///Retrieves the pointer(simply a symbol) of the provided `name`.
    pub fn get_symbol(&self, name: &str) -> Option<SymbolPointer> {
        self.symbols_resolver.retrieve(name).cloned()
    }
}
