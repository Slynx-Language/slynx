use rart::{AdaptiveRadixTree, VectorKey};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
///A pointer to some intern string. This is 56bits for the actual position of the string in the internalized string, and 8bits for it's length
pub struct SymbolPointer(usize);

impl SymbolPointer {
    pub fn new(ptr: usize, length: u8) -> Self {
        Self(ptr << 8 | length as usize)
    }
}

#[derive(Debug, Default)]
///A internalized string is an wrapper over strings to don't allocate too much on the heap.
///The inner string might look something like `CONTENTCONTENT2CONTENT3`, where `CONTENT`, `CONTENT2` and `CONTENT3` are the actual string inserted
pub struct InternalizedString {
    inner: Vec<u8>,
}

impl InternalizedString {
    ///Inserts the provided `s` string and returns it's pointer. Note that it's size cannot be >255
    pub fn insert(&mut self, s: &str) -> SymbolPointer {
        let ptr = self.inner.len();
        let size = s.len();
        self.inner.extend_from_slice(s.as_bytes());
        SymbolPointer::new(ptr, size as u8)
    }
}

///The structure that will be responsible to intern names inside the HIR. It will map a string S to it's symbol
pub struct SymbolsModule {
    names: InternalizedString,
    name_mapping: AdaptiveRadixTree<VectorKey, SymbolPointer>,
}

impl std::default::Default for SymbolsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Index<SymbolPointer> for SymbolsModule {
    type Output = str;
    fn index(&self, ptr: SymbolPointer) -> &Self::Output {
        let size = ptr.0 & 0xff;
        let ptr = ptr.0 >> 8;
        unsafe { str::from_utf8_unchecked(&self.names.inner[ptr..ptr + size]) }
    }
}

impl SymbolsModule {
    pub fn new() -> Self {
        Self {
            names: InternalizedString::default(),
            name_mapping: AdaptiveRadixTree::new(),
        }
    }
    pub fn intern(&mut self, s: &str) -> SymbolPointer {
        if let Some(ptr) = self.name_mapping.get(s) {
            *ptr
        } else {
            let ptr = self.names.insert(s);
            self.name_mapping.insert(s, ptr);
            ptr
        }
    }
    ///Retrieves the pointer of the string on this module.
    pub fn retrieve(&self, s: &str) -> Option<&SymbolPointer> {
        self.name_mapping.get(s)
    }
}

impl std::fmt::Debug for SymbolsModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = f
            .debug_struct("SymbolsModule")
            .field("names", &self.names)
            .finish();
        write!(f, "{result:?}",)
    }
}
