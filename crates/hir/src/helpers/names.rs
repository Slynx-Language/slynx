use common::SymbolPointer;

use crate::SlynxHir;

impl SlynxHir {
    /// Returns the source-level name string for the given symbol pointer.
    pub fn get_name(&self, ptr: SymbolPointer) -> &str {
        self.modules.symbols_resolver.get_name(ptr)
    }
}
