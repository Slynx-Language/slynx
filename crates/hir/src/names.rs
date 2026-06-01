use crate::{Result, SlynxHir, SymbolPointer, TypeId, VariableId};

use common::Span;
//file specific to implement things related to name resolution
impl SlynxHir {
    pub fn intern_name(&mut self, name: &str) -> SymbolPointer {
        self.modules.intern_name(name)
    }

    ///Creates a mutable variable with the given `name` and `ty`
    pub(crate) fn create_mutable_variable(
        &mut self,
        symbol: SymbolPointer,
        ty: TypeId,
        span: &Span,
    ) -> Result<VariableId> {
        self.modules.create_variable(symbol, true, ty, span)
    }
    ///Creates a imutable variable with the given `name` and `ty`
    pub(crate) fn create_variable(
        &mut self,
        symbol: SymbolPointer,
        ty: TypeId,
        span: &Span,
    ) -> Result<VariableId> {
        self.modules.create_variable(symbol, false, ty, span)
    }
}
