use crate::intermediate::id::{ValueId, VarId};

#[derive(Debug)]
pub enum IntermediatePlace {
    ///A reference to a variable
    Local(VarId),
    Field {
        parent: VarId,
        field: usize,
    },
}

#[derive(Debug)]
pub struct IntermediateInstruction {
    pub kind: IntermediateInstructionKind,
}

#[derive(Debug)]
pub enum IntermediateInstructionKind {
    ///Allocates(creates) a new function
    Alloc(VarId),
    ///Moves the expression in `value` into `target`
    Move {
        target: IntermediatePlace,
        value: ValueId,
    },
    ///Reads a variable with provided id
    Read(VarId),
    ///Returns with the value on the provided id
    Ret(ValueId),
}

impl IntermediateInstruction {
    pub fn alloc(varialbe_id: VarId) -> Self {
        Self {
            kind: IntermediateInstructionKind::Alloc(varialbe_id),
        }
    }
    pub fn mov(target: IntermediatePlace, value: ValueId) -> Self {
        Self {
            kind: IntermediateInstructionKind::Move { target, value },
        }
    }
    pub fn read(varialbe_id: VarId) -> Self {
        Self {
            kind: IntermediateInstructionKind::Read(varialbe_id),
        }
    }
    pub fn ret(varialbe_id: ValueId) -> Self {
        Self {
            kind: IntermediateInstructionKind::Ret(varialbe_id),
        }
    }
}
