use smallvec::SmallVec;

use crate::IRTypeId;

pub struct IRStruct {
    fields: SmallVec<[IRTypeId; 8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
///A reference to some struct on the IR
pub struct IRStructId(pub usize);

impl IRStruct {
    ///Creates a new empty struct
    pub fn new() -> Self {
        IRStruct {
            fields: SmallVec::new(),
        }
    }
    ///Inserts the provided `field` onto this struct's fields
    pub fn insert_field(&mut self, field: IRTypeId) {
        self.fields.push(field);
    }
}