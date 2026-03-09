use crate::IRStructId;

/// Logical identifier for a type inside IR type storage.
/// 
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IRType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Struct(IRStructId)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
///A reference to some type on the IR
pub struct IRTypeId(pub usize);

impl IRTypeId {
    ///Creates a new `IRTypeId` from a raw usize
    pub fn from_raw(raw: usize) -> Self {
        IRTypeId(raw)
    }
}