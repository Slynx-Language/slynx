use common::Operator;
use frontend::hir::definitions::HirExpression;

use crate::{
    IRError, IRPointer, IRTypeId, Instruction, Label, Operand, SlynxIR, Value, ir::temp::TempIRData,
};

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
    pub fn handle_binary_expression(
        &mut self,
        lhs: &HirExpression,
        rhs: &HirExpression,
        op: &Operator,
        temp: &mut TempIRData,
    ) -> Result<Value, IRError> {
        let lhs_value = self.get_value_for(&*lhs, temp)?;
        let rhs_value = self.get_value_for(&*rhs, temp)?;
        let lty = self.get_type_of_value(lhs_value.clone(), temp);
        let rty = self.get_type_of_value(rhs_value.clone(), temp);

        if lty != rty {}

        let bin_instruction = match op {
            common::Operator::Add => {
                self.get_add_instruction(lhs_value, rhs_value, lty, temp.current_label())
            }
            common::Operator::Sub => {
                self.get_sub_instruction(lhs_value, rhs_value, lty, temp.current_label())
            }
            common::Operator::Star => {
                self.get_mul_instruction(lhs_value, rhs_value, lty, temp.current_label())
            }
            common::Operator::Slash => {
                self.get_div_instruction(lhs_value, rhs_value, lty, temp.current_label())
            }
            common::Operator::Equals => {
                self.get_cmp_instruction(lhs_value, rhs_value, lty, temp.current_label())
            }
            common::Operator::GreaterThan => {
                self.get_gt_instruction(lhs_value, rhs_value, lty, temp.current_label())
            }
            common::Operator::GreaterThanOrEqual => {
                self.get_gte_instruction(lhs_value, rhs_value, lty, temp.current_label())
            }
            common::Operator::LessThan => {
                self.get_lt_instruction(lhs_value, rhs_value, lty, temp.current_label())
            }
            common::Operator::LessThanOrEqual => {
                self.get_lte_instruction(lhs_value, rhs_value, lty, temp.current_label())
            }
            common::Operator::LogicAnd => {
                let bool_type = self.types.bool_type();
                let thenlabel = self.insert_label(temp.current_function(), "and_then");
                let elselabel = self.insert_label(temp.current_function(), "and_else");
                let endlabel = {
                    let label = self.insert_label(temp.current_function(), "and_end");
                    self.get_label_mut(label.clone())
                        .insert_arguments(&[bool_type]);
                    label //$end_label(bool):
                };

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::cbr(
                        lhs_value,
                        thenlabel.clone(),
                        elselabel.clone(),
                        IRPointer::null(),
                        IRPointer::null(),
                        bool_type,
                    ),
                ); //cbr lhs, then, else;
                temp.set_current_label(thenlabel);
                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(endlabel.clone(), rhs_value.with_length(), bool_type), //br and_end(rhs)
                );

                temp.set_current_label(elselabel);

                let false_value = self.insert_operands(&[Operand::Bool(false)]);
                let false_value = self.insert_value(Value::Raw(false_value));

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(endlabel.clone(), false_value.with_length(), bool_type),
                ); //br and_end(false)

                temp.set_current_label(endlabel.clone());
                let value = self.get_label(endlabel).get_argument_value(0);
                let value = self.insert_value(value);
                self.insert_instruction(temp.current_label(), Instruction::raw(value, bool_type))
            }
            common::Operator::LogicOr => {
                let bool_type = self.types.bool_type();
                let elselabel = self.insert_label(temp.current_function(), "or_else");
                let endlabel = {
                    let label = self.insert_label(temp.current_function(), "or_end");
                    self.get_label_mut(label.clone())
                        .insert_arguments(&[bool_type]);
                    label //$end_label(bool):
                };

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::cbr(
                        lhs_value,
                        endlabel.clone(),
                        elselabel.clone(),
                        IRPointer::null(),
                        IRPointer::null(),
                        bool_type,
                    ),
                ); //cbr lhs, end, else;
                temp.set_current_label(elselabel);

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(endlabel.clone(), rhs_value.with_length(), bool_type), //br and_end(rhs)
                );

                let false_value = self.insert_operands(&[Operand::Bool(false)]);
                let false_value = self.insert_value(Value::Raw(false_value));

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(endlabel.clone(), false_value.with_length(), bool_type),
                ); //br and_end(false)

                temp.set_current_label(endlabel.clone());
                let value = self.get_label(endlabel).get_argument_value(0);
                let value = self.insert_value(value);
                self.insert_instruction(temp.current_label(), Instruction::raw(value, bool_type))
            }
        };

        Ok(Value::Instruction(bin_instruction))
    }
}
