use frontend::hir::{VariableId, definitions::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind}};

use crate::{IRTypeId, SlynxIR, ir::{model::{Context, IRPointer, Instruction, Label, Operand}, temp::TempIRData}};

impl SlynxIR {
    
    pub fn get_basic_operand(&mut self, value: &HirExpression) -> Option<(IRPointer<Operand,1>, IRTypeId)> {
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
            
            HirExpressionKind::Identifier(v) => {
                let operand = Operand::Variable(IRPointer::null());
                (self.insert_operands(&[operand]), self.types.int_type()) //must replace with identifier logic
            }
            _ => return None,
        };
        Some(out)
    }
    
    ///Inserts a slice of operands into the IR and returns a pointer to the first operand.
    pub fn insert_operands<const N: usize>(&mut self, operands: &[Operand; N]) -> IRPointer<Operand, N> {
        let operand_ptr = self.operands.len();
        let out = IRPointer::new(operand_ptr, operands.len());
        self.operands.extend_from_slice(operands);
        out
    }
    
    ///Returns an instruction pointer for the given expression.
    pub fn get_instruction_for(&self, expr: &HirExpression, temp:&TempIRData) -> IRPointer<Instruction, 1> {
        let ptr = self.instructions.len();
        let out = IRPointer::new(ptr, 1);
        let instruction = match &expr.kind {
            HirExpressionKind::Bool(_) | HirExpressionKind::Float(_) | HirExpressionKind::Int(_) => {
                let (operand, optype) = self.get_basic_operand(expr).unwrap();
                Instruction::raw(operand,optype) 
            }
            HirExpressionKind::FunctionCall{ name, args } => {
                let func = {
                    let func = temp.get_function(*name);
                    debug_assert!(func.len() == 1);
                    func.with_length::<1>()
                };
                let ret_ty = self.return_type_of_context(func);
                let mut operands = Vec::with_capacity(args.len());
                for arg in args {
                    operands.push(self.get_basic_operand(arg));
                }
                Instruction::call(func, ret_ty, args)
            }
            v => unreachable!("{v:?} not implemented"),
        };
        self.instructions.push(instruction);
        out
    }
    
    pub fn insert_instruction(&mut self, label: IRPointer<Label,1>, instr: Instruction) {
        self.instructions.push(instr);
        let label = self.get_label_mut(label);
        label.insert_instruction();
    }
    
    pub fn initialize_function(&mut self, ir: IRPointer<Context,1>, statements: &[HirStatement], args: &[VariableId], temp: &mut TempIRData) {
        temp.set_current_function(ir.clone());
        let ctx = self.get_context(ir.clone());
        let label = self.insert_label(ir.clone(), "entry");
        for statement in statements {
            match &statement.kind {
                HirStatementKind::Variable { name, value } => {
                    let value = self.get_instruction_for(value, temp);
                    self.insert_variable(ir.clone(), value);
                                       
                }
                HirStatementKind::Assign { lhs, value } => {
                    
                }
                
                HirStatementKind::Expression { expr } => {
                }
                HirStatementKind::Return { expr } => {
                }
            }
        }
    }
}