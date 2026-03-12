use crate::ir::model::IRVar;

use super::{IRPointer, instruction::Instruction};

#[derive(Debug, Clone)]
///A label is a named 'piece' of block that has got instructions and can be used to determine values
pub struct Label {
    ///The instructions this label has got. The max limit due to the IRPointer is about 65k instructions per label
    instruction: IRPointer<Instruction>,
    variables: IRPointer<IRVar>
}

impl Label {
    ///Creates a new empty label
    pub fn new() -> Self {
        Self {
            instruction: IRPointer::null(),
            variables: IRPointer::null(),
        }
    }
    
    #[inline]
    ///Returns the label's variable pointer
    pub fn variables(&self) -> IRPointer<IRVar> {
        self.variables.clone()
    }
    
    #[inline]
    ///Returns the label's instruction pointer
    pub fn instruction(&self) -> IRPointer<Instruction> {
        self.instruction.clone()
    }
    #[inline]
    ///Inserts an instruction into the label's instruction list
    pub fn insert_instruction(&mut self) {
        self.instruction.increase_length();
    }
    
    #[inline]
    ///Inserts a variable into the label's variable list
    pub fn insert_variable(&mut self) {
        self.variables.increase_length();
    }
}