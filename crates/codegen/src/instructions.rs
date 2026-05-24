use common::Operator;
use either::Either;
use paste::paste;
use slynx_hir::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind};
use slynx_ir::{IRPointer, Instruction, Slot, Value};

use crate::{Codegen, CodegenError};

macro_rules! impl_bin_expression {
    ($($name:ident),* $(,)?) => {
        paste! {
            $(
                fn [<generate_ $name _instruction>](
                    &mut self,
                    lhs: IRPointer<Value, 1>,
                    rhs: IRPointer<Value, 1>,
                    ty: IRTypeId,
                    label: IRPointer<Label, 1>,
                    map: bool,
                ) -> InstructionPtr {
                    let values = self.insert_value(self.get_value(lhs));
                    self.insert_value(self.get_value(rhs));

                    self.insert_instruction(
                        label,
                        Instruction::$name(ty, values.with_length()),
                        map,
                    )
                }
            )*
        }
    };
}

impl Codegen {
    fn emit_while_statement(
        &mut self,
        condition: &HirExpression,
        body: &[HirStatement],
    ) -> Result<Option<IRPointer<Instruction, 1>>, CodegenError> {
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
        let cond_value = self.generate_value_for(condition, temp)?;

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
            self.generate_statement(stmt, temp)?;
        }

        self.insert_instruction(
            temp.current_label(),
            Instruction::br(cond_label, IRPointer::null(), bool_type),
            true,
        );

        temp.set_current_label(end_label);

        Ok(None)
    }

    fn emit_assign_statement(
        &mut self,
        lhs: &HirExpression,
        value: &HirExpression,
    ) -> Result<Option<IRPointer<Instruction, 1>>, CodegenError> {
        let value_ptr = self.generate_value_for(value, temp)?;

        match &lhs.kind {
            HirExpressionKind::Identifier(id) => {
                let value_slot = temp
                    .get_variable(*id)
                    .expect("Variable not found for assignment");
                let slot = match &*self.get_value(value_slot) {
                    ValueKind::Slot(slot) => *slot,
                    other => panic!("Expected Slot value for variable, got {:?}", other),
                };
                self.write(slot, value_ptr, temp);
            }
            HirExpressionKind::FieldAccess {
                expr: parent,
                field_index,
            } => {
                let parent_value = self.generate_value_for(parent, temp)?;
                let parent_ty = self.get_type_of_value(parent_value);

                let values_ptr = {
                    let parent = self.get_value(parent_value);
                    let value = self.get_value(value_ptr);
                    self.insert_values(&[parent, value])
                };
                self.insert_instruction(
                    temp.current_label(),
                    Instruction::setfield(*field_index, values_ptr.with_length(), parent_ty),
                    true,
                );
            }
            _ => unreachable!("LHS of assignment must be Identifier or FieldAccess"),
        }
        Ok(None)
    }

    ///Emits IR instructions for the given HIR `statement`
    pub(crate) fn lower_statement(
        &mut self,
        statement: &HirStatement,
    ) -> Result<Option<IRPointer<Instruction, 1>>, CodegenError> {
        match &statement.kind {
            HirStatementKind::While { condition, body } => {
                self.emit_while_statement(condition, body, temp)
            }

            HirStatementKind::Variable { name, value } => {
                let vty = self.get_ir_type(&value.ty, temp)?;
                let (slotvalue, slotptr) = self.allocate(vty, temp);
                let value = self.generate_value_for(value, temp)?;

                self.write(slotptr, value, temp);

                temp.add_variable(*name, slotvalue);
                Ok(None)
            }
            HirStatementKind::Assign { lhs, value } => self.emit_assign_statement(lhs, value, temp),

            HirStatementKind::Expression { expr } => {
                self.lower_expression(expr)?;
                Ok(None)
            }
            HirStatementKind::Return { expr } => {
                let val = self.lower_expression(expr)?;
                let ty = self.get_type_of_value(val);
                let instruction =
                    self.insert_instruction(temp.current_label(), Instruction::ret(val, ty), true);
                let instruction = self.dereference_instruction_ptr(instruction).with_length();
                Ok(Some(instruction))
            }
        }
    }

    impl_bin_expression!(
        add, sub, mul, div, cmp, gt, gte, lt, lte, and, or, shr, shl, xor
    );

    fn generate_logic_and_instruction(
        &mut self,
        lhs_value: IRPointer<Value, 1>,
        rhs_value: IRPointer<Value, 1>,
    ) -> InstructionPtr {
        let bool_type = self.types.bool_type();
        let thenlabel = self.insert_label(temp.current_function(), "and_then");
        let elselabel = self.insert_label(temp.current_function(), "and_else");
        let endlabel = {
            let label = self.insert_label(temp.current_function(), "and_end");
            self.get_label_mut(label).insert_arguments(&[bool_type]);
            label
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
        );
        temp.set_current_label(thenlabel);
        self.insert_instruction(
            temp.current_label(),
            Instruction::br(endlabel, rhs_value.with_length(), bool_type),
            true,
        );

        temp.set_current_label(elselabel);

        let false_value = self.insert_operands(&[Operand::Bool(false)]);
        let false_value = self.insert_value(self.create_raw_value(false_value));

        self.insert_instruction(
            temp.current_label(),
            Instruction::br(endlabel, false_value.with_length(), bool_type),
            true,
        );

        temp.set_current_label(endlabel);
        let value = self.get_label(endlabel).get_argument_value(0);
        let value = self.insert_value(value);
        self.insert_instruction(
            temp.current_label(),
            Instruction::raw(value, bool_type),
            true,
        )
    }

    fn generate_logic_or_instruction(
        &mut self,
        lhs_value: IRPointer<Value, 1>,
        rhs_value: IRPointer<Value, 1>,
    ) -> InstructionPtr {
        let bool_type = self.types.bool_type();
        let elselabel = self.insert_label(temp.current_function(), "or_else");
        let endlabel = {
            let label = self.insert_label(temp.current_function(), "or_end");
            self.get_label_mut(label).insert_arguments(&[bool_type]);
            label
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
        );
        temp.set_current_label(elselabel);

        self.insert_instruction(
            temp.current_label(),
            Instruction::br(endlabel, rhs_value.with_length(), bool_type),
            true,
        );

        let false_value = self.insert_operands(&[Operand::Bool(false)]);
        let false_value = self.insert_value(self.create_raw_value(false_value));

        self.insert_instruction(
            temp.current_label(),
            Instruction::br(endlabel, false_value.with_length(), bool_type),
            true,
        );

        temp.set_current_label(endlabel);
        let value = self.get_label(endlabel).get_argument_value(0);
        let value = self.insert_value(value);
        self.insert_instruction(
            temp.current_label(),
            Instruction::raw(value, bool_type),
            true,
        )
    }

    pub(crate) fn handle_binary_expression(
        &mut self,
        lhs: &HirExpression,
        rhs: &HirExpression,
        op: &Operator,
    ) -> Result<Value, IRError> {
        let lhs_value = self.generate_value_for(lhs, temp)?;
        let rhs_value = self.generate_value_for(rhs, temp)?;
        let lty = self.get_type_of_value(lhs_value);

        let bin_instruction = match op {
            common::Operator::And => self.generate_and_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),
            common::Operator::Or => {
                self.generate_or_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::RightShift => self.generate_shr_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),
            common::Operator::LeftShift => self.generate_shl_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),
            common::Operator::Xor => self.generate_xor_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),

            common::Operator::Add => self.generate_add_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),
            common::Operator::Sub => self.generate_sub_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),
            common::Operator::Star => self.generate_mul_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),
            common::Operator::Slash => self.generate_div_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),
            common::Operator::Equals => self.generate_cmp_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),
            common::Operator::GreaterThan => {
                self.generate_gt_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::GreaterThanOrEqual => self.generate_gte_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),
            common::Operator::LessThan => {
                self.generate_lt_instruction(lhs_value, rhs_value, lty, temp.current_label(), false)
            }
            common::Operator::LessThanOrEqual => self.generate_lte_instruction(
                lhs_value,
                rhs_value,
                lty,
                temp.current_label(),
                false,
            ),
            common::Operator::LogicAnd => {
                self.generate_logic_and_instruction(lhs_value, rhs_value, temp)
            }
            common::Operator::LogicOr => {
                self.generate_logic_or_instruction(lhs_value, rhs_value, temp)
            }
        };
        let bin_instruction = self.dereference_instruction_ptr(bin_instruction);
        let ty = self.get_instruction_by_pointer(bin_instruction)[0].value_type;
        Ok(Value::new_instruction(bin_instruction.with_length(), ty))
    }

    pub(crate) fn allocate(&mut self, ty: IRTypeId) -> (IRPointer<Value, 1>, IRPointer<Slot, 1>) {
        let ptr = self.slots.len();
        self.slots.push(Slot { ty });
        let out = IRPointer::new(ptr, 1);
        let _ = self.insert_instruction(temp.current_label(), Instruction::allocate(out, ty), true);

        (self.insert_value(Value::new_slot(out, ty)), out)
    }

    pub(crate) fn write(
        &mut self,
        slot: IRPointer<Slot, 1>,
        value: IRPointer<Value, 1>,
    ) -> IRPointer<IRPointer<Instruction, 1>, 1> {
        let slot_type = self.slots[slot.ptr()].ty;
        let v = self
            .insert_instruction(
                temp.current_label(),
                Instruction::write(slot_type, slot, value),
                true,
            )
            .unwrap_left();
        IRPointer::new(v.ptr(), v.len())
    }
}
