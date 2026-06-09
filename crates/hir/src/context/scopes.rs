use std::ops::{Deref, DerefMut, Index, IndexMut};

use dashmap::DashMap;

use crate::{SymbolPointer, VariableId};

/// A single lexical scope that maps symbol names to variable IDs.
#[derive(Debug)]
pub struct HIRScope {
    ///A map to a name to an id. This can be used to save variables for example
    names: DashMap<SymbolPointer, VariableId>,
    mutables: boxcar::Vec<VariableId>,
}

impl Default for HIRScope {
    fn default() -> Self {
        Self::new()
    }
}

impl HIRScope {
    /// Creates a new, empty [`HIRScope`].
    pub fn new() -> Self {
        Self {
            mutables: boxcar::Vec::new(),
            names: DashMap::new(),
        }
    }

    ///Inserts the provided `symbol` on this scope
    pub fn create_name(&self, symbol: SymbolPointer, var: VariableId, mutable: bool) {
        self.names.insert(symbol, var);
        if mutable {
            self.mutables.push(var);
        }
    }

    ///Retrieves the id of the provided `name` on the scope
    pub fn get_name(&self, name: &SymbolPointer) -> Option<VariableId> {
        self.names.get(name).map(|name| *name.value())
    }
}

#[derive(Debug, Default)]
///A Context made with the intent of managing data inside scopes. Note that everything on this scope will have affect on the last defined scope.
///So when entering a new scope, it means all functions will have effect on this new scope. This struct always derefs to the last active scope
pub struct ScopeContext {
    scopes: Vec<HIRScope>,
}

impl ScopeContext {
    /// Creates a new [`ScopeContext`] with an initial global scope already pushed.
    pub fn new() -> Self {
        let mut out = Self::default();
        out.enter_scope();
        out
    }

    ///Retrieves how many scopes there are
    pub fn len(&self) -> usize {
        self.scopes.len()
    }
    ///Returns if this scope is empty. Not necessarily useful, just because clippy proclaims about
    pub fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }
    ///Enter a new scope and returns a mutable reference to it.
    pub fn enter_scope(&mut self) -> &mut HIRScope {
        self.scopes.push(HIRScope::new());
        self.scopes.last_mut().unwrap()
    }
    ///Exit the current scope and returns a mutable reference to it.
    pub fn exit_scope(&mut self) -> Option<HIRScope> {
        self.scopes.pop()
    }
    ///Returns an iterator that iterates over the most recent scope, until the global one
    pub fn iter(&self) -> ScopeContextIterator<'_> {
        ScopeContextIterator {
            scope_context: self,
            index: self.len(),
        }
    }
}

pub struct ScopeContextIterator<'a> {
    scope_context: &'a ScopeContext,
    index: usize,
}

impl<'a> Iterator for ScopeContextIterator<'a> {
    type Item = &'a HIRScope;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 0 {
            None
        } else {
            self.index -= 1;
            Some(&self.scope_context[self.index])
        }
    }
}

impl Deref for ScopeContext {
    type Target = HIRScope;
    fn deref(&self) -> &Self::Target {
        self.scopes.last().unwrap()
    }
}
impl DerefMut for ScopeContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.scopes.last_mut().unwrap()
    }
}

impl Index<usize> for ScopeContext {
    type Output = HIRScope;
    fn index(&self, index: usize) -> &Self::Output {
        &self.scopes[index]
    }
}

impl IndexMut<usize> for ScopeContext {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.scopes[index]
    }
}
