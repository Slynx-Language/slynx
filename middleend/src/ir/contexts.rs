use frontend::hir::{
    VariableId,
    definitions::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind},
};

use crate::{
    IRError, IRTypeId, SlynxIR,
    ir::{
        model::{Context, IRPointer, Instruction, Label, Operand, Value},
        temp::TempIRData,
    },
};

impl SlynxIR {
    ///Gets the new operands that are required to be inserted by the provided `value`. The final operand is the one with the actual value. Note that this function
    ///might add instructions to the current context since an Expression can be a complex task
    pub fn get_operand(
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

    pub fn insert_dynamic_operands(&mut self, operands: &[Operand]) -> IRPointer<Operand, 0> {
        let ptr = self.operands.len();
        let out = IRPointer::new(ptr, operands.len());
        self.operands.extend_from_slice(operands);
        out
    }

    ///Inserts a slice of operands into the IR and returns a pointer to the first operand.
    pub fn insert_operands<const N: usize>(
        &mut self,
        operands: &[Operand; N],
    ) -> IRPointer<Operand, N> {
        let operand_ptr = self.operands.len();
        let out = IRPointer::new(operand_ptr, operands.len());
        self.operands.extend_from_slice(operands);
        out
    }

    ///Gets a value based on its `ptr`
    pub fn get_value(&self, ptr: IRPointer<Value, 1>) -> Value {
        self.values[ptr.ptr()].clone()
    }

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
    ///Returns an instruction pointer for the given expression.
    pub fn get_value_for(
        &mut self,
        expr: &HirExpression,
        temp: &mut TempIRData,
    ) -> Result<IRPointer<Value, 1>, IRError> {
        let value = match &expr.kind {
            HirExpressionKind::Bool(_)
            | HirExpressionKind::Float(_)
            | HirExpressionKind::Int(_) => {
                let (operand, optype) = self.get_operand(expr, temp).unwrap();
                let value = self.insert_value(Value::Raw(operand));
                let instruction =
                    self.insert_instruction(temp.current_label(), Instruction::raw(value, optype));
                Value::Instruction(instruction)
            }
            HirExpressionKind::FunctionCall { name, args } => {
                let func = {
                    let func = temp.get_function(*name);
                    debug_assert!(func.len() == 1);
                    func.with_length::<1>()
                };
                let ret_ty = self.return_type_of_context(func.clone());
                let mut operands = Vec::with_capacity(args.len());

                for arg in args {
                    let value = self.get_value_for(arg, temp)?;
                    operands.push(value);
                }
                let ptr = self.operands.len();
                for operand in operands.iter() {
                    let op = self.operands[operand.ptr()].clone();
                    self.operands.push(op);
                }
                let ptr = IRPointer::new(ptr, operands.len());
                let instruction = self
                    .insert_instruction(temp.current_label(), Instruction::call(func, ret_ty, ptr));
                Value::Instruction(instruction)
            }
            HirExpressionKind::Binary { lhs, op, rhs } => {
                let lhs_value = self.get_value_for(&*lhs, temp)?;
                let rhs_value = self.get_value_for(&*rhs, temp)?;
                let lty = self.get_type_of_value(lhs_value.clone(), temp);
                #[cfg(debug_assertions)]
                {
                    let rty = self.get_type_of_value(rhs_value.clone(), temp);
                    debug_assert!(lty == rty);
                    match lty {
                        _ if lty == self.types.int_type() || lty == self.types.float_type() => {}
                        _ => panic!("Binary operation should be made only by int and float types"),
                    };
                }

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
                        let temp_var = self.insert_value(value)
                        let thenlabel = self.insert_label(temp.current_function(), "and_then");
                        let elselabel = self.insert_label(temp.current_function(), "and_else");
                        let endlabel = {
                            let label = self.insert_label(temp.current_function(), "and_end");
                            self.get_label_mut(endlabel).insert_arguments(&[self.types.bool_type()]);
                            label //$end_label(bool):
                        };
                        let bool_type = self.types.bool_type();
                        
                        let instruction = self.insert_instruction(
                            temp.current_label(),
                            Instruction::cbr(
                                lhs_value,
                                thenlabel,
                                elselabel,
                                IRPointer::null(),
                                IRPointer::null(),
                                bool_type,
                            ),
                        ); //cbr lhs, then, else;
                        temp.set_current_label(thenlabel);
                        let rhs = self.get_value_for(rhs, temp)?;
                        self.insert_instruction(
                            temp.current_label(),
                            Instruction::br(endlabel, rhs.with_length(), bool_type), //br and_end(rhs)
                        );

                        temp.set_current_label(elselabel);
                        
                        let false_value = self.insert_operands(&[Operand::Bool(false)]));
                        let false_value = self.insert_value(Value::Raw(false_value));
                        
                        self.insert_instruction(temp.current_label(), Instruction::br(endlabel, false_value.with_length(), bool_type)); //br and_end(false)

                        temp.set_current_label(endlabel);
                        let value = self.get_label(endlabel).get_argument_value(0);
                        let label_ty = self.get_label(endlabel).arguments()[0];
                        let value = self.insert_value(value);
                        self.insert_instruction(temp.current_label(), Instruction::ret(value, label_ty))
                    }
                    btn => panic!("{btn:?} unimplemented"),
                };
                Value::Instruction(bin_instruction)
            }
            HirExpressionKind::Identifier(id) => {
                if let Some(value) = temp.get_variable(*id) {
                    self.get_value(value)
                } else {
                    return Err(IRError::UnrecognizedVariable(*id));
                }
            }
            v => unreachable!("{v:?} not implemented"),
        };
        Ok(self.insert_value(value))
    }

    pub fn insert_instruction(
        &mut self,
        label: IRPointer<Label, 1>,
        instr: Instruction,
    ) -> IRPointer<Instruction, 1> {
        self.instructions.push(instr);
        let label = self.get_label_mut(label);
        label.insert_instruction();
        label.instruction().ptr_to_last()
    }

    pub fn initialize_function(
        &mut self,
        ir: IRPointer<Context, 1>,
        statements: &[HirStatement],
        args: &[VariableId],
        temp: &mut TempIRData,
    ) -> Result<(), IRError> {
        temp.set_current_function(ir.clone());
        let ptr = IRPointer::new(self.values.len(), args.len());
        for (idx, _) in args.iter().enumerate() {
            self.insert_value(Value::FuncArg(idx));
        }
        temp.set_function_args(args, ptr);
        let _ = self.get_context(ir.clone());
        let _ = self.insert_label(ir.clone(), "entry");
        for statement in statements {
            match &statement.kind {
                HirStatementKind::Variable { name, value } => {
                    let value = self.get_value_for(value, temp)?;
                    temp.add_variable(*name, value);
                }
                HirStatementKind::Assign { .. } => {}

                HirStatementKind::Expression { .. } => {}
                HirStatementKind::Return { expr } => {
                    let val = self.get_value_for(expr, temp)?;
                    let ty = self.get_type_of_value(val.clone(), temp);
                    self.insert_instruction(temp.current_label(), Instruction::ret(val, ty));
                }
            }
        }
        Ok(())
    }
}
