use smallvec::SmallVec;
mod irtype;
mod structs;
pub use structs::*;
pub use irtype::*;

pub const BUILTIN_TYPES: &[IRType] = &[IRType::I8, IRType::U8, IRType::I16, IRType::U16, IRType::I32, IRType::U32, IRType::I64, IRType::U64, IRType::F32, IRType::F64];

pub struct IRTypes {
    types: Vec<IRType>,
    structs: Vec<IRStruct>
}

impl IRTypes {
    pub fn new() -> Self {
        Self {
            types: BUILTIN_TYPES.to_vec(),
            structs: Vec::new(),
        }
    }
    ///Creates a new empty struct and returns its type ID
    pub fn create_empty_struct(&mut self) -> IRTypeId {
        self.structs.push(IRStruct::new());
        let out = self.types.len();
        self.types.push(IRType::Struct(IRStructId(out)));
        IRTypeId(out)
    }
}