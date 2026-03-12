use frontend::hir::{VariableId, definitions::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind}};

use crate::{IRTypeId, SlynxIR, ir::{model::{Context, IRPointer, Instruction, Label, Operand}, temp::TempIRData}};

impl SlynxIR {
    
    pub fn get_basic_operand(&mut self, value: &HirExpression) -> Option<(IRPointer<Operand>, IRTypeId)> {
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
    
    pub fn insert_operands(&mut self, operands: &[Operand]) -> IRPointer<Operand> {
        let operand_ptr = self.operands.len();
        let out = IRPointer::new(operand_ptr, operands.len());
        self.operands.extend_from_slice(operands);
        out
    }
    
    pub fn get_instruction_for(&self, expr: &HirExpression, temp:&TempIRData) -> IRPointer<Instruction> {
        let ptr = self.instructions.len();
        let out = IRPointer::new(ptr, 1);
        let instruction = match &expr.kind {
            HirExpressionKind::Bool(_) | HirExpressionKind::Float(_) | HirExpressionKind::Int(_) => {
                let (operand, optype) = self.get_basic_operand(expr).unwrap();
                Instruction::raw(operand,optype) 
            }
            HirExpressionKind::FunctionCall{ name, args } => {
                let func = temp.get_function(*name);
                let ret_ty = self.return_type_of_context(func);
                let mut operands = Vec::with_capacity(args.len());
                for arg in args {
                    operands.push(self.get_basic_operand(arg, temp));
                }
                Instruction::call(func, ret_ty, args)
            }
            v => unreachable!("{v:?} not implemented"),
        };
        self.instructions.push(instruction);
        out
    }
    
    pub fn insert_instruction(&mut self, label: IRPointer<Label>, instr: Instruction) {
        self.instructions.push(instr);
        let label = self.get_label_mut(label);
        label.insert_instruction();
    }
    
    pub fn initialize_function(&mut self, ir: IRPointer<Context>, statements: &[HirStatement], args: &[VariableId], temp: &mut TempIRData) {
        temp.set_current_function(ir);
        let ctx = self.get_context(ir.clone());
        let label = self.insert_label(ir, "entry");
        for statement in statements {
            match &statement.kind {
                HirStatementKind::Variable { name, value } => {
                    let value = self.get_instruction_for(value);
                    self.insert_variable(ir, value);
                                       
                }
                HirStatementKind::Assign { lhs, value } => {
                    let lhs = self.get_operand(lhs);
                    let value = self.get_operand(value);
                    ctx.add_instruction();
                }
                
                HirStatementKind::Expression { expr } => {
                }
                HirStatementKind::Return { expr } => {
                }
            }
        }
    }
}