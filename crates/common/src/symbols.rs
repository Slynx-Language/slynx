use std::{hash::Hash, marker::PhantomData};

use rart::{AdaptiveRadixTree, VectorKey};
///A pointer to some intern string. This is 48bits for the actual position of the string in the internalized string, and 16bits for it's length
#[derive(Debug)]
pub struct SymbolPointer<T>(u64, PhantomData<T>);

impl<T> SymbolPointer<T> {
    pub fn new(ptr: u64, length: u16) -> Self {
        Self(ptr << 16 | length as u64, PhantomData)
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

///A internalized string is an wrapper over strings to don't allocate too much on the heap.
///The inner string might look something like `CONTENTCONTENT2CONTENT3`, where `CONTENT`, `CONTENT2` and `CONTENT3` are the actual string inserted
#[derive(Debug, Default)]
pub struct InternalizedString {
    inner: Vec<u8>,
}

impl InternalizedString {
    ///Inserts the provided `s` string and returns it's pointer. Note that it's size cannot be >65535
    pub fn insert<T>(&mut self, s: &str) -> SymbolPointer<T> {
        let ptr = self.inner.len() as u64;
        let size = s.len();
        self.inner.extend_from_slice(s.as_bytes());
        SymbolPointer::new(ptr, size as u16)
    }
}

///The structure that will be responsible to intern names inside the HIR. It will map a string S to it's symbol
pub struct SymbolsModule<Ctx> {
    names: InternalizedString,
    name_mapping: AdaptiveRadixTree<VectorKey, SymbolPointer<Ctx>>,
}

impl<Ctx> std::default::Default for SymbolsModule<Ctx> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Ctx> std::ops::Index<SymbolPointer<Ctx>> for SymbolsModule<Ctx> {
    type Output = str;
    fn index(&self, ptr: SymbolPointer<Ctx>) -> &Self::Output {
        let size = ptr.0 as usize & 0xffff;
        let ptr = ptr.0 as usize >> 16;
        unsafe { std::str::from_utf8_unchecked(&self.names.inner[ptr..ptr + size]) }
    }
}

impl<Ctx> SymbolsModule<Ctx> {
    pub fn new() -> Self {
        Self {
            names: InternalizedString::default(),
            name_mapping: AdaptiveRadixTree::new(),
        }
    }
    pub fn intern(&mut self, s: &str) -> SymbolPointer<Ctx> {
        if let Some(ptr) = self.name_mapping.get(s) {
            *ptr
        } else {
            let ptr = self.names.insert(s);
            self.name_mapping.insert(s, ptr);
            ptr
        }
    }
    ///Retrieves the pointer of the string on this module.
    pub fn retrieve(&self, s: &str) -> Option<&SymbolPointer<Ctx>> {
        self.name_mapping.get(s)
    }

    pub fn get_name(&self, ptr: SymbolPointer<Ctx>) -> &str {
        &self[ptr]
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
