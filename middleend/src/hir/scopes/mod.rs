use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::hir::scopes::scope::HIRScope;

mod scope;
#[derive(Debug, Default)]
///A module made with the intent of managing data inside scopes. Note that everything on this scope will have affect on the last defined scope.
///So when entering a new scope, it means all functions will have effect on this new scope. This struct always derefs to the last active scope
pub struct ScopeModule {
    scopes: Vec<HIRScope>,
}

impl ScopeModule {
    pub fn new() -> Self {
        let mut out = Self::default();
        out.enter_scope();
        out
    }

    ///Retrieves how many scopes there are
    pub fn len(&mut self) -> usize {
        self.scopes.len()
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
}

impl Deref for ScopeModule {
    type Target = HIRScope;
    fn deref(&self) -> &Self::Target {
        self.scopes.last().unwrap()
    }
}
impl DerefMut for ScopeModule {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.scopes.last_mut().unwrap()
    }
}

impl Index<usize> for ScopeModule {
    type Output = HIRScope;
    fn index(&self, index: usize) -> &Self::Output {
        &self.scopes[index]
    }
}

impl IndexMut<usize> for ScopeModule {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.scopes[index]
    }
}
