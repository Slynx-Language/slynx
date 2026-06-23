use std::ops::Deref;

use common::{FrontendSymbol, SymbolsModule};
use dashmap::DashMap;

use crate::{SymbolPointer, VariableId};

/// Wraps a [`SymbolsModule`] and additionally tracks the source-level symbol for each variable.
#[derive(Debug)]
pub struct SymbolsResolver<'a> {
    module: &'a SymbolsModule<FrontendSymbol>,
    /// Tracks the original source-level symbol for each variable id.
    variable_names: DashMap<VariableId, SymbolPointer>,
}

impl<'a> Deref for SymbolsResolver<'a> {
    type Target = SymbolsModule<FrontendSymbol>;
    fn deref(&self) -> &Self::Target {
        &self.module
    }
}

impl<'a> SymbolsResolver<'a> {
    /// Creates a new [`SymbolsResolver`] wrapping the given [`SymbolsModule`].
    pub fn new(module: &'a SymbolsModule<FrontendSymbol>) -> Self {
        Self {
            module,
            variable_names: DashMap::new(),
        }
    }

    /// Associates the given variable ID with its source-level symbol pointer.
    pub fn create_variable(&self, id: VariableId, symbol: SymbolPointer) {
        self.variable_names.insert(id, symbol);
    }

    /// Returns the map from variable IDs to their source-level symbol pointers.
    pub fn variables(&self) -> &DashMap<VariableId, SymbolPointer> {
        &self.variable_names
    }
    /// Returns a reference to the underlying [`SymbolsModule`].
    pub fn symbols_module(&self) -> &SymbolsModule<FrontendSymbol> {
        &self.module
    }
}
