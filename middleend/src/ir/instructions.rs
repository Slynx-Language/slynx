use crate::{IRPointer, IRTypeId, Instruction, Label, SlynxIR, Value};

impl SlynxIR {
    ///Adds the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub fn get_add_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
    ) -> IRPointer<Instruction, 1> {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::add(ty, values.with_length()))
    }
    ///Subtracts the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub fn get_sub_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
    ) -> IRPointer<Instruction, 1> {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::sub(ty, values.with_length()))
    }
    ///Multiplies the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub fn get_mul_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
    ) -> IRPointer<Instruction, 1> {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::mul(ty, values.with_length()))
    }
    ///Divides the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub fn get_div_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
    ) -> IRPointer<Instruction, 1> {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::div(ty, values.with_length()))
    }
    ///Compares the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub fn get_cmp_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
    ) -> IRPointer<Instruction, 1> {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::cmp(ty, values.with_length()))
    }
    ///Greater than the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub fn get_gt_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
    ) -> IRPointer<Instruction, 1> {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::gt(ty, values.with_length()))
    }
    ///Greater than the the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub fn get_gte_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
    ) -> IRPointer<Instruction, 1> {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::gte(ty, values.with_length()))
    }
    ///Less than the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub fn get_lt_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
    ) -> IRPointer<Instruction, 1> {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::lt(ty, values.with_length()))
    }
    ///Less than or equal the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub fn get_lte_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
    ) -> IRPointer<Instruction, 1> {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::lte(ty, values.with_length()))
    }
}
