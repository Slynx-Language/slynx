use crate::{IRViewer, Instruction, InstructionType};

impl<'a> IRViewer<'a, Instruction> {
    pub fn raw_type(&self) -> &InstructionType {
        &self.value().instruction_type
    }
}
