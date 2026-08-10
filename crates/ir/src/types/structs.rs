use std::ops::{Deref, DerefMut};

use bitflags::bitflags;
use smallvec::SmallVec;

use crate::{IRTypeId, SymbolPointer};

bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct IRStructFlags: u64 {
        ///Flag to tell that this is struct represent a null type
        const NULLABLE = 0b1;
    }
}

#[derive(Debug, Default)]
pub struct IRStruct {
    fields: SmallVec<[IRTypeId; 8]>,
    name: Option<SymbolPointer>,
    flags: IRStructFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
///A reference to some struct on the IR
pub struct IRStructId(pub usize);

impl IRStruct {
    ///Creates a new empty struct
    pub fn new(name: Option<SymbolPointer>) -> Self {
        IRStruct {
            fields: SmallVec::new(),
            name,
            flags: IRStructFlags::empty(),
        }
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
