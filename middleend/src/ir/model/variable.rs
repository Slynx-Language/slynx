use crate::ir::model::{IRPointer, Instruction};

#[derive(Debug, Clone)]
pub struct IRVar{
    value: IRPointer<Instruction>
}

impl IRVar {
    pub fn new(value: IRPointer<Instruction>) -> Self {
        Self { value }
    }
    
    pub fn value(&self) -> IRPointer<Instruction> {
        self.value.clone()
    }
}