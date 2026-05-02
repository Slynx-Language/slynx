use common::Operator;
use either::Either;
use slynx_hir::model::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind};

use crate::{
    IRError, IRPointer, IRTypeId, Instruction, Label, Operand, Slot, SlynxIR, Value,
    ir::temp::TempIRData,
};

pub type InstructionPtr = Either<IRPointer<IRPointer<Instruction>, 1>, IRPointer<Instruction>>;

impl SlynxIR {
    ///Gets one(or more) instructions to operate with the given `statement`
    pub(crate) fn get_instruction(
        &mut self,
        statement: &HirStatement,
        temp: &mut TempIRData,
    ) -> Result<Option<IRPointer<Instruction, 1>>, IRError> {
        match &statement.kind {
            HirStatementKind::While { condition, body } => {
                let cond_label = self.insert_label(temp.current_function(), "while_cond");
                let body_label = self.insert_label(temp.current_function(), "while_body");
                let end_label = self.insert_label(temp.current_function(), "while_end");

                let bool_type = self.types.bool_type();

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(cond_label, IRPointer::null(), bool_type),
                    true,
                );

                temp.set_current_label(cond_label);
                let cond_value = self.get_value_for(condition, temp)?;

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::cbr(
                        cond_value,
                        body_label,
                        end_label,
                        IRPointer::null(),
                        IRPointer::null(),
                        bool_type,
                    ),
                    true,
                );

                temp.set_current_label(body_label);
                for stmt in body {
                    self.get_instruction(stmt, temp)?;
                }

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(cond_label, IRPointer::null(), bool_type),
                    true,
                );

                temp.set_current_label(end_label);

                Ok(None)
            }

            HirStatementKind::Variable { name, value } => {
                let vty = self.get_ir_type(&value.ty, temp)?;
                let (slotvalue, slotptr) = self.allocate(vty, temp);
                let value = self.get_value_for(value, temp)?;

                self.write(slotptr, value, temp);

                temp.add_variable(*name, slotvalue);
                Ok(None)
            }
            HirStatementKind::Assign { lhs, value } => {
                let value_ptr = self.get_value_for(value, temp)?;

                match &lhs.kind {
                    HirExpressionKind::Identifier(id) => {
                        let value_slot = temp
                            .get_variable(*id)
                            .expect("Variable not found for assignment");
                        let slot = match self.get_value(value_slot) {
                            Value::Slot(slot) => slot,
                            other => panic!("Expected Slot value for variable, got {:?}", other),
                        };
                        self.write(slot, value_ptr, temp);
                    }
                    HirExpressionKind::FieldAccess {
                        expr: parent,
                        field_index,
                    } => {
                        let parent_value = self.get_value_for(parent, temp)?;
                        let parent_ty = self.get_type_of_value(parent_value, temp);

                        // SetField: create a new struct with the field modified

                        let values_ptr = {
                            let parent = self.get_value(parent_value);
                            let value = self.get_value(value_ptr);
                            self.insert_values(&[parent, value])
                        };
                        self.insert_instruction(
                            temp.current_label(),
                            Instruction::setfield(
                                *field_index,
                                values_ptr.with_length(),
                                parent_ty,
                            ),
                            true,
                        );
                    }
                    _ => unreachable!("LHS of assignment must be Identifier or FieldAccess"),
                }
                Ok(None)
            }

            HirStatementKind::Expression { expr } => {
                self.get_value_for(expr, temp)?;
                Ok(None)
            }
            HirStatementKind::Return { expr } => {
                let val = self.get_value_for(expr, temp)?;
                let ty = self.get_type_of_value(val, temp);
                let instruction =
                    self.insert_instruction(temp.current_label(), Instruction::ret(val, ty), true);
                let instruction = self.dereference_instruction_ptr(instruction).with_length();
                Ok(Some(instruction))
            }
        }
    }
    ///Inserts the provided `instr` on the given `label`. If `map` is `true` then the label will be able to read it when compiling, thus, otherwise, its just an intermediate instruction. If `map=true`, returns the reference to the mapped instruction, otherwise, returns the reference to the instruction itself(without being mapped)
    pub(crate) fn insert_instruction(
        &mut self,
        label: IRPointer<Label, 1>,
        instr: Instruction,
        map: bool,
    ) -> InstructionPtr {
        let ptr = self.instructions.len();
        self.instructions.push(instr);
        if map {
            let outptr = self.instruction_pointers.len();
            self.instruction_pointers.push(IRPointer::new(ptr, 1));
            let label = self.get_label_mut(label);
            label.insert_instruction();
            Either::Left(IRPointer::new(outptr, 1))
        } else {
            Either::Right(IRPointer::new(ptr, 1))
        }
    }
    ///Adds the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_add_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::add(ty, values.with_length()), map)
    }
    ///Subtracts the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_sub_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::sub(ty, values.with_length()), map)
    }
    ///Multiplies the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_mul_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::mul(ty, values.with_length()), map)
    }
    ///Divides the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_div_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::div(ty, values.with_length()), map)
    }
    ///Compares the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_cmp_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::cmp(ty, values.with_length()), map)
    }
    ///Greater than the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_gt_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::gt(ty, values.with_length()), map)
    }
    ///Greater than the the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_gte_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::gte(ty, values.with_length()), map)
    }
    ///Less than the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_lt_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::lt(ty, values.with_length()), map)
    }
    ///Less than or equal the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_lte_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::lte(ty, values.with_length()), map)
    }
    ///Greater than the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_and_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::and(ty, values.with_length()), map)
    }
    ///Greater than the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_or_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::or(ty, values.with_length()), map)
    }
    ///Greater than the the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_shr_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        if self.types.is_negative_int(ty) {
            self.insert_instruction(label, Instruction::ashr(ty, values.with_length()), map)
        } else {
            self.insert_instruction(label, Instruction::shr(ty, values.with_length()), map)
        }
    }
    ///Less than the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_shl_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::shl(ty, values.with_length()), map)
    }
    ///Less than or equal the provided `lhs` and `rhs` as a binary on the provided `label`. Its idealized to be the current one
    pub(crate) fn get_xor_instruction(
        &mut self,
        lhs: IRPointer<Value, 1>,
        rhs: IRPointer<Value, 1>,
        ty: IRTypeId,
        label: IRPointer<Label, 1>,
        map: bool,
    ) -> InstructionPtr {
        let values = self.insert_value(self.get_value(lhs));
        self.insert_value(self.get_value(rhs));
        self.insert_instruction(label, Instruction::xor(ty, values.with_length()), map)
    }
    pub(crate) fn handle_binary_expression(
        &mut self,
        lhs: &HirExpression,
        rhs: &HirExpression,
        op: &Operator,
        temp: &mut TempIRData,
    ) -> Result<Value, IRError> {
        let lhs_value = self.get_value_for(lhs, temp)?;
        let rhs_value = self.get_value_for(rhs, temp)?;
        let lty = self.get_type_of_value(lhs_value, temp);

        let bin_instruction = match op {
            common::Operator::And => {
                self.get_and_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::Or => {
                self.get_or_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::RightShift => {
                self.get_shr_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::LeftShift => {
                self.get_shl_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::Xor => {
                self.get_xor_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }

            common::Operator::Add => {
                self.get_add_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::Sub => {
                self.get_sub_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::Star => {
                self.get_mul_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::Slash => {
                self.get_div_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::Equals => {
                self.get_cmp_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::GreaterThan => {
                self.get_gt_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::GreaterThanOrEqual => {
                self.get_gte_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::LessThan => {
                self.get_lt_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::LessThanOrEqual => {
                self.get_lte_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::LogicAnd => {
                let bool_type = self.types.bool_type();
                let thenlabel = self.insert_label(temp.current_function(), "and_then");
                let elselabel = self.insert_label(temp.current_function(), "and_else");
                let endlabel = {
                    let label = self.insert_label(temp.current_function(), "and_end");
                    self.get_label_mut(label).insert_arguments(&[bool_type]);
                    label //$end_label(bool):
                };

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::cbr(
                        lhs_value,
                        thenlabel,
                        elselabel,
                        IRPointer::null(),
                        IRPointer::null(),
                        bool_type,
                    ),
                    true,
                ); //cbr lhs, then, else;
                temp.set_current_label(thenlabel);
                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(endlabel, rhs_value.with_length(), bool_type), //br and_end(rhs)
                    true,
                );

                temp.set_current_label(elselabel);

                let false_value = self.insert_operands(&[Operand::Bool(false)]);
                let false_value = self.insert_value(Value::Raw(false_value));

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(endlabel, false_value.with_length(), bool_type),
                    true,
                ); //br and_end(false)

                temp.set_current_label(endlabel);
                let value = self.get_label(endlabel).get_argument_value(0);
                let value = self.insert_value(value);
                self.insert_instruction(
                    temp.current_label(),
                    Instruction::raw(value, bool_type),
                    true,
                )
            }
            common::Operator::LogicOr => {
                let bool_type = self.types.bool_type();
                let elselabel = self.insert_label(temp.current_function(), "or_else");
                let endlabel = {
                    let label = self.insert_label(temp.current_function(), "or_end");
                    self.get_label_mut(label).insert_arguments(&[bool_type]);
                    label //$end_label(bool):
                };

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::cbr(
                        lhs_value,
                        endlabel,
                        elselabel,
                        IRPointer::null(),
                        IRPointer::null(),
                        bool_type,
                    ),
                    true,
                ); //cbr lhs, end, else;
                temp.set_current_label(elselabel);

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(endlabel, rhs_value.with_length(), bool_type), //br and_end(rhs)
                    true,
                );

                let false_value = self.insert_operands(&[Operand::Bool(false)]);
                let false_value = self.insert_value(Value::Raw(false_value));

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(endlabel, false_value.with_length(), bool_type),
                    true,
                ); //br and_end(false)

                temp.set_current_label(endlabel);
                let value = self.get_label(endlabel).get_argument_value(0);
                let value = self.insert_value(value);
                self.insert_instruction(
                    temp.current_label(),
                    Instruction::raw(value, bool_type),
                    true,
                )
            }
        };

        let bin_instruction = self.dereference_instruction_ptr(bin_instruction);
        Ok(Value::Instruction(bin_instruction.with_length()))
    }

    pub(crate) fn allocate(
        &mut self,
        ty: IRTypeId,
        temp: &TempIRData,
    ) -> (IRPointer<Value, 1>, IRPointer<Slot, 1>) {
        let ptr = self.slots.len();
        self.slots.push(Slot { ty });
        let out = IRPointer::new(ptr, 1);
        let _ = self.insert_instruction(temp.current_label(), Instruction::allocate(out, ty), true);

        (self.insert_value(Value::Slot(out)), out)
    }

    pub(crate) fn write(
        &mut self,
        slot: IRPointer<Slot, 1>,
        value: IRPointer<Value, 1>,
        temp: &TempIRData,
    ) -> IRPointer<IRPointer<Instruction>, 1> {
        let slot_type = self.slots[slot.ptr()].ty;
        self.insert_instruction(
            temp.current_label(),
            Instruction::write(slot_type, slot, value),
            true,
        )
        .unwrap_left()
    }
}
