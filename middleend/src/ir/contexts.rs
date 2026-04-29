use frontend::hir::{
    VariableId,
    model::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind},
};

use crate::{
    IRError, IRTypeId, SlynxIR,
    ir::{
        model::{Context, IRPointer, Instruction, Label, Operand, Value},
        temp::TempIRData,
    },
};

impl SlynxIR {
    fn lower_if_branch(
        &mut self,
        branch: &[HirStatement],
        end_label: IRPointer<Label, 1>,
        temp: &mut TempIRData,
    ) -> Result<Option<IRTypeId>, IRError> {
        for (idx, statement) in branch.iter().enumerate() {
            if idx == branch.len() - 1
                && let HirStatementKind::Expression { ref expr } = statement.kind
            {
                let value = self.get_value_for(expr, temp)?;
                let ty = self.get_type_of_value(value, temp);

                // The merge label carries the value produced by each branch.
                // Its argument type is the branch result type, not the condition type.
                if self.get_label(end_label).arguments().is_empty() {
                    self.get_label_mut(end_label).add_argument(ty);
                }

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(end_label, value.with_length(), ty),
                    true,
                );
                return Ok(Some(ty));
            }

            if self.get_instruction(statement, temp)?.is_some() {
                return Ok(None);
            }
        }

        self.insert_instruction(
            temp.current_label(),
            Instruction::br(end_label, IRPointer::null(), self.types.void_type()),
            true,
        );
        Ok(None)
    }

    ///Gets the new operands that are required to be inserted by the provided `value`. The final operand is the one with the actual value. Note that this function
    ///might add instructions to the current context since an Expression can be a complex task
    pub(crate) fn get_operand(
        &mut self,
        value: &HirExpression,
        _temp: &mut TempIRData,
    ) -> Option<(IRPointer<Operand, 1>, IRTypeId)> {
        let out = match value.kind {
            HirExpressionKind::Bool(i) => {
                let operand = Operand::Bool(i);
                (self.insert_operands(&[operand]), self.types.bool_type())
            }
            HirExpressionKind::Int(i) => {
                let operand = Operand::Int(i as i64);
                (self.insert_operands(&[operand]), self.types.int_type())
            }
            HirExpressionKind::Float(f) => {
                let operand = Operand::Float(f as f64);
                (self.insert_operands(&[operand]), self.types.float_type())
            }
            _ => return None,
        };
        Some(out)
    }

    ///Inserts a slice of operands into the IR and returns a pointer to the first operand.
    pub(crate) fn insert_operands<const N: usize>(
        &mut self,
        operands: &[Operand; N],
    ) -> IRPointer<Operand, N> {
        let operand_ptr = self.operands.len();
        let out = IRPointer::new(operand_ptr, operands.len());
        self.operands.extend_from_slice(operands);
        out
    }

    ///Gets a value based on its `ptr`
    pub(crate) fn get_value(&self, ptr: IRPointer<Value, 1>) -> Value {
        self.values[ptr.ptr()].clone()
    }

    ///Returns an instruction pointer for the given expression.
    pub(crate) fn get_value_for(
        &mut self,
        expr: &HirExpression,
        temp: &mut TempIRData,
    ) -> Result<IRPointer<Value, 1>, IRError> {
        let value = match &expr.kind {
            HirExpressionKind::Tuple(vetor) => {
                let values_ptrs = vetor
                    .iter()
                    .map(|e| self.get_value_for(e, temp))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut element_types = Vec::with_capacity(values_ptrs.len());
                for ptr in &values_ptrs {
                    let ty = self.get_type_of_value(*ptr, temp);
                    element_types.push(ty);
                }
                let values = values_ptrs
                    .iter()
                    .map(|v| self.get_value(*v))
                    .collect::<Vec<_>>();
                let values_ptr = self.insert_values(&values);

                let ty = self.types.create_or_get_tuple(element_types);
                Value::StructLiteral(ty, values_ptr)
            }
            HirExpressionKind::StringLiteral(v) => {
                let handle_idx = self.strings.intern(v);
                let operand = Operand::String(handle_idx);
                let operand_ptr = self.insert_operands(&[operand]);
                let value = self.insert_value(Value::Raw(operand_ptr));
                let instruction = self.insert_instruction(
                    temp.current_label(),
                    Instruction::raw(value, self.types.str_type()),
                    false,
                );
                let ptr = self.dereference_instruction_ptr(instruction);
                Value::Instruction(ptr.with_length())
            }
            HirExpressionKind::Bool(_)
            | HirExpressionKind::Float(_)
            | HirExpressionKind::Int(_) => {
                let (operand, optype) = self.get_operand(expr, temp).unwrap();
                let value = self.insert_value(Value::Raw(operand));
                let instruction = self.insert_instruction(
                    temp.current_label(),
                    Instruction::raw(value, optype),
                    false,
                );
                Value::Instruction(self.dereference_instruction_ptr(instruction).with_length())
            }
            HirExpressionKind::FunctionCall { name, args } => {
                let func = {
                    let func = temp.get_function(*name);
                    debug_assert!(func.len() == 1);
                    func.with_length::<1>()
                };
                let ret_ty = self.return_type_of_context(func);
                let mut operands = Vec::with_capacity(args.len());

                for arg in args {
                    let value = self.get_value_for(arg, temp)?;
                    operands.push(value);
                }
                let ptr = self.operands.len();
                let ptr = IRPointer::new(ptr, operands.len());
                let instruction = self.insert_instruction(
                    temp.current_label(),
                    Instruction::call(func, ret_ty, ptr),
                    false,
                );
                Value::Instruction(self.dereference_instruction_ptr(instruction).with_length())
            }
            HirExpressionKind::Binary { lhs, op, rhs } => {
                self.handle_binary_expression(lhs, rhs, op, temp)?
            }
            HirExpressionKind::Identifier(id) => {
                if let Some(value) = temp.get_variable(*id) {
                    self.get_value(value)
                } else {
                    return Err(IRError::UnrecognizedVariable(*id));
                }
            }
            HirExpressionKind::Object { name, fields } => {
                let ty = temp.get_type(*name)?;
                let values = {
                    let fields = fields
                        .iter()
                        .map(|v| self.get_value_for(v, temp))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.insert_values(
                        &fields
                            .iter()
                            .map(|v| self.get_value(*v))
                            .collect::<Vec<_>>(),
                    )
                };
                let strts = self.insert_instruction(
                    temp.current_label(),
                    Instruction::struct_literal(ty, values),
                    false,
                );
                Value::Instruction(self.dereference_instruction_ptr(strts).with_length())
            }
            HirExpressionKind::FieldAccess { expr, field_index } => {
                let value = self.get_value_for(expr, temp)?;
                let ty = self.get_type_of_value(value, temp);
                let i = self.insert_instruction(
                    temp.current_label(),
                    Instruction::getfield(*field_index, value, ty),
                    false,
                );
                Value::Instruction(self.dereference_instruction_ptr(i).with_length())
            }
            HirExpressionKind::Component { name, values } => {
                self.get_component_expression(*name, values, temp)?
            }
            HirExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let value = self.get_value_for(condition, temp)?;
                let then_label = self.insert_label(temp.current_function(), "then_label");
                let else_label = self.insert_label(temp.current_function(), "else_label");
                let end_label = self.insert_label(temp.current_function(), "end_label");
                let condition_ty = self.get_type_of_value(value, temp);

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::cbr(
                        value,
                        then_label,
                        else_label,
                        IRPointer::null(),
                        IRPointer::null(),
                        condition_ty,
                    ),
                    true,
                );

                {
                    let next = self.get_next_mapeable_instruction_ptr();
                    let mut fresh = next.with_length::<0>();
                    fresh.set_length(0);
                    self.get_label_mut(then_label)
                        .set_instructions_pointer(fresh);
                }

                temp.set_current_label(then_label);
                self.lower_if_branch(then_branch, end_label, temp)?;

                {
                    let next = self.get_next_mapeable_instruction_ptr();
                    let mut fresh = next.with_length::<0>();
                    fresh.set_length(0);
                    self.get_label_mut(else_label)
                        .set_instructions_pointer(fresh);
                }

                temp.set_current_label(else_label);
                let else_branch = else_branch.as_deref().unwrap_or(&[]);
                self.lower_if_branch(else_branch, end_label, temp)?;

                {
                    let next = self.get_next_mapeable_instruction_ptr();
                    let mut fresh = next.with_length::<0>();
                    fresh.set_length(0);
                    self.get_label_mut(end_label)
                        .set_instructions_pointer(fresh);
                }

                temp.set_current_label(end_label);
                let end_label = self.get_label(end_label);
                if end_label.arguments().is_empty() {
                    Value::Void
                } else {
                    end_label.get_argument_value(0)
                }
            }
            HirExpressionKind::Specialized(_) => {
                unimplemented!("Not implemented IR for specialized components");
            }
        };
        Ok(self.insert_value(value))
    }

    pub(crate) fn initialize_function(
        &mut self,
        ir: IRPointer<Context, 1>,
        statements: &[HirStatement],
        args: &[VariableId],
        temp: &mut TempIRData,
    ) -> Result<(), IRError> {
        temp.set_current_function(ir);

        {
            let label = self.insert_label(ir, "entry");
            self.get_context_mut(ir).set_label_ptr(label.with_length()); //must do so because this gets the next avaible position to labels
            let next_instruction = self.get_next_mapeable_instruction_ptr();
            let mut ptr = next_instruction.with_length();
            ptr.set_length(ptr.len() - 1);
            self.get_label_mut(label).set_instructions_pointer(ptr);
            temp.set_current_label(label);
        }

        let ptr = IRPointer::new(self.values.len(), args.len());
        for (idx, _) in args.iter().enumerate() {
            self.insert_value(Value::FuncArg(idx));
        }

        temp.set_function_args(args, ptr);
        for statement in statements {
            self.get_instruction(statement, temp)?;
        }

        Ok(())
    }
}
