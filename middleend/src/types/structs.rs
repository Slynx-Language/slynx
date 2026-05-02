use common::SymbolPointer;
use smallvec::SmallVec;

use crate::IRTypeId;

#[derive(Debug, Default)]
pub struct IRStruct {
    fields: SmallVec<[IRTypeId; 8]>,
    name: Option<SymbolPointer>,
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
