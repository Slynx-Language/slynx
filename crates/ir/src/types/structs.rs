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
    pub struct IRStructFlags: u64 {
        ///Flag to tell that this is struct represent a null type
        const NULLABLE = 0b1;
    }
}

#[derive(Debug, Default, Clone)]
pub struct IRStruct {
    fields: SmallVec<[IRTypeId; 8]>,
    name: Option<SymbolPointer>,
    flags: IRStructFlags,
}

///A reference to some struct on the IR
pub type IRStructId = DedupPoolId<IRStruct>;

impl IRStruct {
    ///Creates a new empty struct
    pub fn new(name: Option<SymbolPointer>) -> Self {
        IRStruct {
            fields: SmallVec::new(),
            name,
            flags: IRStructFlags::empty(),
        }
    }

    ///Sets the flags of this struct, returning itself
    pub fn with_flags(mut self, flags: IRStructFlags) -> Self {
        self.flags = flags;
        self
    }

    ///Sets the fields of this struct, returning itself
    pub fn with_fields(mut self, fields: impl IntoIterator<Item = IRTypeId>) -> Self {
        self.fields.extend(fields);
        self
    }

    ///Inserts the provided `field` onto this struct's fields
    pub fn insert_field(&mut self, field: IRTypeId) {
        self.fields.push(field);
    }

    pub fn get_fields(&self) -> &[IRTypeId] {
        &self.fields
    }

    pub fn name(&self) -> Option<SymbolPointer> {
        self.name
    }
}

impl PartialEq for IRStruct {
    fn eq(&self, other: &Self) -> bool {
        match (self.name, other.name) {
            //Named structs are dedup'd by name alone. Their fields are allowed
            //to be mutated after insertion (e.g. object structs that get their
            //fields populated in a later lowering phase).
            (Some(a), Some(b)) => a == b,
            //Anonymous structs (tuples) are fully formed on insertion and are
            //dedup'd by their contents.
            (None, None) => self.fields == other.fields && self.flags == other.flags,
            _ => false,
        }
    }
}
impl Eq for IRStruct {}

impl Hash for IRStruct {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.name {
            Some(name) => {
                0u8.hash(state);
                name.hash(state);
            }
            None => {
                1u8.hash(state);
                self.fields.hash(state);
                self.flags.hash(state);
            }
        }
    }
}

impl Deref for IRStruct {
    type Target = IRStructFlags;
    fn deref(&self) -> &Self::Target {
        &self.flags
    }
}
impl DerefMut for IRStruct {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.flags
    }
}
