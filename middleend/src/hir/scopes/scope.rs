use std::collections::HashMap;

use crate::hir::{VariableId, symbols::SymbolPointer};

#[derive(Debug)]
pub struct HIRScope {
    ///A map to a name to an id. This can be used to save variables for example
    names: HashMap<SymbolPointer, VariableId>,
    mutables: Vec<VariableId>,
}

impl HIRScope {
    pub fn new() -> Self {
        Self {
            mutables: Vec::new(),
            names: HashMap::new(),
        }
    }

    ///Inserts the provided `symbol` on this scope
    pub fn insert_name(&mut self, symbol: SymbolPointer, var: VariableId, mutable: bool) {
        self.names.insert(symbol, var);
        if mutable {
            self.mutables.push(var);
        }
    }

    ///Retrieves the id of the provided `name` on the scope
    pub fn retrieve_name(&self, name: &SymbolPointer) -> Option<&VariableId> {
        self.names.get(name)
    }
}
