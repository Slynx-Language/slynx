use crate::{HIRError, HirSymbol, Result, SlynxHir, SymbolPointer, TypeId, VariableId};

use common::Span;

//file specific to implement things related to name resolution
impl SlynxHir {
    pub fn resolve_name(&self, symbol: SymbolPointer, span: &Span) -> Result<HirSymbol> {
        match true {
            _ if let Ok(out) = self.get_variable(symbol, span) => Ok(HirSymbol::Variable(out)),
            _ if let Some((out, _)) = self.modules.get_declaration_by_name(&symbol) => {
                Ok(HirSymbol::Declaration(out))
            }
            _ => Err(HIRError::name_unrecognized(symbol, *span)),
        }
    }

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
