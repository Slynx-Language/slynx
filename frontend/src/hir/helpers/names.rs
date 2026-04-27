use common::SymbolPointer;

use crate::hir::SlynxHir;

impl SlynxHir {
    pub fn get_name(&self, ptr: SymbolPointer) -> &str {
        self.modules.symbols_resolver.get_name(ptr)
    }
}
