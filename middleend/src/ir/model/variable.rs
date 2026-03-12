use crate::ir::model::{IRPointer, Instruction};

#[derive(Debug, Clone)]
pub struct IRVar{
    value: IRPointer<Instruction,1>
}

impl IRVar {
    pub fn new(value: IRPointer<Instruction,1>) -> Self {
        Self { value }
    }
    
    pub fn value(&self) -> IRPointer<Instruction,1> {
        self.value.clone()
    }
}