use crate::{HIRError, Result, SlynxHir, SymbolPointer, TypeId, VariableId, module_loader::FileId};

use common::Span;
//file specific to implement things related to name resolution
impl SlynxHir {
    ///Creates a mutable variable with the given `name` and `ty`
    pub(crate) fn create_mutable_variable(
        &self,
        file: FileId,
        symbol: SymbolPointer,
        ty: TypeId,
        span: &Span,
    ) -> Result<VariableId> {
        let out = self.get_file(file).create_variable(symbol, span, false)?;
        self.types_module.insert_variable(out, ty);
        self.symbols_resolver.create_variable(out, symbol);
        Ok(out)
    }
    ///Creates a imutable variable with the given `name` and `ty`
    pub(crate) fn create_variable(
        &self,
        file: FileId,
        symbol: SymbolPointer,
        ty: TypeId,
        span: &Span,
    ) -> Result<VariableId> {
        let out = self.get_file(file).create_variable(symbol, span, false)?;
        self.types_module.insert_variable(out, ty);
        self.symbols_resolver.create_variable(out, symbol);
        Ok(out)
    }
    ///Tries to retrieve a variable with the provided `name` on the current active scope
    pub fn get_variable(
        &self,
        fileid: FileId,
        symbol: SymbolPointer,
        span: &Span,
    ) -> Result<VariableId> {
        if let Some(variable) = self.get_file(fileid).scopes.get_name(&symbol) {
            Ok(variable)
        } else {
            Err(HIRError::name_unrecognized(symbol, *span))
        }
    }
    ///Retrieves the pointer(simply a symbol) of the provided `name`.
    pub fn get_symbol(&self, name: &str) -> Option<SymbolPointer> {
        self.symbols_resolver.retrieve(name)
    }
}
