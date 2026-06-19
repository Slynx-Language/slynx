use std::{hash::Hash, marker::PhantomData};

use lasso::{Spur, ThreadedRodeo};

#[derive(Debug)]
pub struct FrontendSymbol;

///A pointer to some intern string. This is 48bits for the actual position of the string in the internalized string, and 16bits for it's length
#[derive(Debug)]
pub struct SymbolPointer<T>(Spur, PhantomData<T>);

impl<T> SymbolPointer<T> {
    pub fn new(spur: Spur) -> Self {
        Self(spur, PhantomData)
    }
}

impl<T> Clone for SymbolPointer<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SymbolPointer<T> {}

impl<T> PartialEq for SymbolPointer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for SymbolPointer<T> {}

impl<T> Hash for SymbolPointer<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

///The structure that will be responsible to intern names inside the HIR. It will map a string S to it's symbol
pub struct SymbolsModule<Ctx> {
    names: lasso::ThreadedRodeo,
    phantom: PhantomData<Ctx>,
}

impl<Ctx> std::default::Default for SymbolsModule<Ctx> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Ctx> SymbolsModule<Ctx> {
    pub fn new() -> Self {
        Self {
            names: ThreadedRodeo::new(),
            phantom: PhantomData,
        }
    }
    pub fn intern(&self, s: &str) -> SymbolPointer<Ctx> {
        SymbolPointer::new(self.names.get_or_intern(s))
    }
    ///Retrieves the pointer of the string on this module.
    pub fn retrieve(&self, s: &str) -> Option<SymbolPointer<Ctx>> {
        self.names.get(s).map(|s| SymbolPointer::new(s))
    }

    pub fn get_name(&self, ptr: SymbolPointer<Ctx>) -> &str {
        self.names.resolve(&ptr.0)
    }
}

impl<Ctx> std::fmt::Debug for SymbolsModule<Ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = f
            .debug_struct("SymbolsModule")
            .field("names", &self.names)
            .finish();
        write!(f, "{result:?}",)
    }
}
