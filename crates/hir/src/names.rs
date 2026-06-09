use crate::{Result, SlynxHir, SymbolPointer, TypeId, VariableId, module_loader::FileId};

use common::Span;
//file specific to implement things related to name resolution
impl SlynxHir {
    ///Creates a mutable variable with the given `name` and `ty`
    pub(crate) fn create_mutable_variable(
        &mut self,
        file: FileId,
        symbol: SymbolPointer,
        ty: TypeId,
        span: &Span,
    ) -> Result<VariableId> {
        let out = self
            .get_file_mut(file)
            .create_variable(symbol, span, false)?;
        self.types_module.insert_variable(out, ty);
        self.symbols_resolver.create_variable(out, symbol);
        Ok(out)
    }
    ///Creates a imutable variable with the given `name` and `ty`
    pub(crate) fn create_variable(
        &mut self,
        file: FileId,
        symbol: SymbolPointer,
        ty: TypeId,
        span: &Span,
    ) -> Result<VariableId> {
        let out = self
            .get_file_mut(file)
            .create_variable(symbol, span, false)?;
        self.types_module.insert_variable(out, ty);
        self.symbols_resolver.create_variable(out, symbol);
        Ok(out)
    }
}
