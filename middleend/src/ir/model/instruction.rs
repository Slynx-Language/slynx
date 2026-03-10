use crate::IRTypeId;

use super::{IRPointer};

///An operand that is used on a instruction
pub struct Operand {}

///
pub enum InstructionType {
    
}

///A instruction that determines the compiler something that it should execute
pub struct Instruction {
    operands: IRPointer<Operand>,
    instruction_type: InstructionType,
    value_type: IRTypeId
}
