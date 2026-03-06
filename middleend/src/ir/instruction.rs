use crate::IRPointer;

///An operand that is used on a instruction
pub struct Operand {}

///A instruction that determines the compiler something that it should execute
pub struct Instruction {
    operands: IRPointer<Operand>,
}
