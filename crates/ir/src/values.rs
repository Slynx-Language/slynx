use crate::{IRPointer, Operand, SlynxIR};

impl SlynxIR {
    ///Inserts a slice of operands into the IR and returns a pointer to the first operand.
    pub fn create_operands(&mut self, operands: &[Operand]) -> IRPointer<Operand> {
        let operand_ptr = self.operands.len();
        let out = IRPointer::new(operand_ptr, operands.len());
        self.operands.extend_from_slice(operands);
        out
    }
    ///Inserts a slice of operands into the IR and returns a pointer to the first operand.
    pub fn create_single_operand(&mut self, operand: Operand) -> IRPointer<Operand, 1> {
        self.create_operands(&[operand]).with_length()
    }
}
