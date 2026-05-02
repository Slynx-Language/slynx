use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub use common::symbols::*;

use crate::VariableId;

/// Wraps a [`SymbolsModule`] and additionally tracks the source-level symbol for each variable.
#[derive(Debug, Default)]
pub struct SymbolsResolver {
    module: SymbolsModule,
    /// Tracks the original source-level symbol for each variable id.
    variable_names: HashMap<VariableId, SymbolPointer>,
}

impl Deref for SymbolsResolver {
    type Target = SymbolsModule;
    fn deref(&self) -> &Self::Target {
        &self.module
    }
}
impl DerefMut for SymbolsResolver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.module
    }
}

impl SymbolsResolver {
    /// Creates a new [`SymbolsResolver`] wrapping the given [`SymbolsModule`].
    pub fn new(module: SymbolsModule) -> Self {
        Self {
            module,
            variable_names: HashMap::new(),
        }
    }

    /// Associates the given variable ID with its source-level symbol pointer.
    pub fn register_variable(&mut self, id: VariableId, symbol: SymbolPointer) {
        self.variable_names.insert(id, symbol);
    }

    /// Returns the map from variable IDs to their source-level symbol pointers.
    pub fn variables(&self) -> &HashMap<VariableId, SymbolPointer> {
        &self.variable_names
    }
    /// Returns a reference to the underlying [`SymbolsModule`].
    pub fn symbols_module(&self) -> &SymbolsModule {
        &self.module
    }
    /// Consumes this resolver and returns the underlying [`SymbolsModule`].
    pub fn get_symbols_module(self) -> SymbolsModule {
        self.module
    }
}
