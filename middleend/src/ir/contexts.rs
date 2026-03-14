use frontend::hir::{
    VariableId,
    definitions::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind},
};

use crate::{
    IRTypeId, SlynxIR,
    ir::{
        model::{Context, IRPointer, Instruction, Label, Operand},
        temp::TempIRData,
    },
};

impl SlynxIR {
    ///Gets the new operands that are required to be inserted by the provided `value`. The final operand is the one with the actual value. Note that this function
    ///might add instructions to the current context since an Expression can be a complex task
    pub fn get_operands(
        &mut self,
        value: &HirExpression,
        temp: &mut TempIRData,
    ) -> Option<(IRPointer<Operand>, IRTypeId)> {
        let out = match value.kind {
            HirExpressionKind::Bool(i) => {
                let operand = Operand::Bool(i);
                (
                    self.insert_operands(&[operand]).with_runtime_length(1),
                    self.types.bool_type(),
                )
            }
            HirExpressionKind::Int(i) => {
                let operand = Operand::Int(i as i64);
                (
                    self.insert_operands(&[operand]).with_runtime_length(1),
                    self.types.int_type(),
                )
            }
            HirExpressionKind::Float(f) => {
                let operand = Operand::Float(f as f64);
                (
                    self.insert_operands(&[operand]).with_runtime_length(1),
                    self.types.float_type(),
                )
            }

            HirExpressionKind::Identifier(v) => {
                let value = temp.get_variable(v)?;
                let operand = Operand::Variable(value);
                (
                    self.insert_operands(&[operand]).with_runtime_length(1),
                    self.types.int_type(),
                ) //must replace with identifier logic
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

    ///Returns an instruction pointer for the given expression.
    pub fn get_instruction_for(
        &mut self,
        expr: &HirExpression,
        temp: &mut TempIRData,
    ) -> IRPointer<Instruction, 1> {
        let ptr = self.instructions.len();
        let out = IRPointer::new(ptr, 1);
        let instruction = match &expr.kind {
            HirExpressionKind::Bool(_)
            | HirExpressionKind::Float(_)
            | HirExpressionKind::Int(_) => {
                let (operand, optype) = self.get_operands(expr, temp).unwrap();
                Instruction::raw(operand.with_length(), optype)
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
                    let Some((operand, _)) = self.get_operands(arg, temp) else {
                        panic!(
                            "Error on trying to get an operand for a function call '{name:?}'. HIR might be malformed"
                        );
                    };
                    operands.push(operand.ptr_to_last());
                }
                let ptr = self.operands.len();
                for operand in operands.iter() {
                    let op = self.operands[operand.ptr()].clone();
                    self.operands.push(op);
                }
                let ptr = IRPointer::new(ptr, operands.len());
                Instruction::call(func, ret_ty, ptr)
            }
            v => unreachable!("{v:?} not implemented"),
        };
        self.instructions.push(instruction);
        out
    }

    pub fn insert_instruction(&mut self, label: IRPointer<Label, 1>, instr: Instruction) {
        self.instructions.push(instr);
        let label = self.get_label_mut(label);
        label.insert_instruction();
    }

    pub fn initialize_function(
        &mut self,
        ir: IRPointer<Context, 1>,
        statements: &[HirStatement],
        args: &[VariableId],
        temp: &mut TempIRData,
    ) {
        temp.set_current_function(ir.clone());
        let ctx = self.get_context(ir.clone());
        let label = self.insert_label(ir.clone(), "entry");
        for statement in statements {
            match &statement.kind {
                HirStatementKind::Variable { name, value } => {
                    let value = self.get_instruction_for(value, temp);
                    self.insert_variable(ir.clone(), value);
                }
                HirStatementKind::Assign { lhs, value } => {}

                HirStatementKind::Expression { expr } => {}
                HirStatementKind::Return { expr } => {}
            }
        }
    }
}
