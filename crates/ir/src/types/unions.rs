use std::{
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};

use bitflags::bitflags;
use common::pool::DedupPoolId;
use smallvec::SmallVec;

use crate::{IRTypeId, SymbolPointer};

bitflags! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct IRUnionFlags: u64 {
        ///Reserved for future use (e.g. a flag to signal the union holds a
        ///nullable payload). Currently only the empty value exists.
        const NONE = 0;
    }
}

#[derive(Debug, Default, Clone)]
pub struct IRUnion {
    variants: SmallVec<[IRTypeId; 8]>,
    name: Option<SymbolPointer>,
    ///Size of the largest variant, so the union's own size is known before IR
    ///emission. Computed when the union is created; a future sizing pass can
    ///override it via [`Self::set_size`].
    size: usize,
    flags: IRUnionFlags,
}

///A reference to some union on the IR
pub type IRUnionId = DedupPoolId<IRUnion>;

impl IRUnion {
    ///Creates a new empty union
    pub fn new(name: Option<SymbolPointer>) -> Self {
        IRUnion {
            variants: SmallVec::new(),
            name,
            size: 0,
            flags: IRUnionFlags::empty(),
        }
    }

    ///Sets the flags of this union, returning itself
    pub fn with_flags(mut self, flags: IRUnionFlags) -> Self {
        self.flags = flags;
        self
    }

    ///Sets the variants of this union, returning itself
    pub fn with_variants(mut self, variants: impl IntoIterator<Item = IRTypeId>) -> Self {
        self.variants.extend(variants);
        self
    }

    ///Sets the size of this union, returning itself
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    ///Inserts the provided `variant` onto this union's variants
    pub fn insert_variant(&mut self, variant: IRTypeId) {
        self.variants.push(variant);
    }

    pub fn get_variants(&self) -> &[IRTypeId] {
        &self.variants
    }

    pub fn name(&self) -> Option<SymbolPointer> {
        self.name
    }

    ///Returns the size of the largest variant of this union
    pub fn size(&self) -> usize {
        self.size
    }

    ///Sets the size of the largest variant of this union
    pub fn set_size(&mut self, size: usize) {
        self.size = size;
    }
}

impl PartialEq for IRUnion {
    fn eq(&self, other: &Self) -> bool {
        match (self.name, other.name) {
            //Named unions are dedup'd by name alone. Their variants are allowed
            //to be mutated after insertion, mirroring struct handling.
            (Some(a), Some(b)) => a == b,
            //Anonymous unions are fully formed on insertion and are dedup'd by
            //their contents.
            (None, None) => self.variants == other.variants && self.flags == other.flags,
            _ => false,
        }
    }
}
impl Eq for IRUnion {}

impl Hash for IRUnion {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.name {
            Some(name) => {
                0u8.hash(state);
                name.hash(state);
            }
            None => {
                1u8.hash(state);
                self.variants.hash(state);
                self.flags.hash(state);
            }
        }
    }
}

impl Deref for IRUnion {
    type Target = IRUnionFlags;
    fn deref(&self) -> &Self::Target {
        &self.flags
    }
}
impl DerefMut for IRUnion {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.flags
    }
}
