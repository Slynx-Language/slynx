use crate::{IRViewer, Instruction, Opcode};

impl<'a> IRViewer<'a, Instruction> {
    pub fn raw_opcode(&self) -> &Opcode {
        &self.value().opcode
    }
}
