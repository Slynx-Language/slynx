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

    ///Returns an instruction pointer for the given expression.
    pub fn get_value_for(
        &mut self,
        expr: &HirExpression,
        temp: &mut TempIRData,
    ) -> IRPointer<Value, 1> {
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
                    let value = self.get_value_for(arg, temp);
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
            v => unreachable!("{v:?} not implemented"),
        };
        self.insert_value(value)
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
                    let value = self.get_value_for(value, temp);
<<<<<<< HEAD
                    temp.add_variable(*name, value);
=======
                    temp.add_variable(*name, self.get_value(value));
>>>>>>> fee1b75 (feat: removed var and added value instead)
                }
                HirStatementKind::Assign { .. } => {}

                HirStatementKind::Expression { .. } => {}
                HirStatementKind::Return { expr } => {
                    let val = self.get_value_for(expr, temp);
                    let ty = self.get_type_of_value(val.clone(), temp);
                    self.insert_instruction(temp.current_label(), Instruction::ret(val, ty));
                }
            }
        }
        Ok(())
    }
}
