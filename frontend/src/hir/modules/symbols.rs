use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub use common::symbols::*;

use crate::hir::VariableId;

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
    pub fn new(module: SymbolsModule) -> Self {
        Self {
            module,
            variable_names: HashMap::new(),
        }
    }

    pub fn register_variable(&mut self, id: VariableId, symbol: SymbolPointer) {
        self.variable_names.insert(id, symbol);
    }

    pub fn variables(&self) -> &HashMap<VariableId, SymbolPointer> {
        &self.variable_names
    }
    pub fn symbols_module(&self) -> &SymbolsModule {
        &self.module
    }
    pub fn get_symbols_module(self) -> SymbolsModule {
        self.module
    }
}
