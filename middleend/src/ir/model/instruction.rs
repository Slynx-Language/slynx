use crate::{IRTypeId, ir::model::{Context, variable::IRVar}};

use super::{IRPointer};

#[derive(Debug, Clone)]
///An operand that is used on a instruction. Mainly it's values that are used on the instructions
pub enum Operand {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Variable(IRPointer<IRVar>),
    Binary { lhs: IRPointer<Operand>, rhs: IRPointer<Operand> },
    
}

#[derive(Debug, Clone)]
///An enum that represents the type of instruction
pub enum InstructionType {
    ///Variant used for raw values. Their actual value is their operand
    RawValue,
    ///Variant used for function calls. The `func` field is the pointer to the function context
    FunctionCall { func: IRPointer<Context> },
}

#[derive(Debug, Clone)]
///A instruction that determines the compiler something that it should execute
pub struct Instruction {
    operands: IRPointer<Operand>,
    instruction_type: InstructionType,
    value_type: IRTypeId
}

impl Instruction {
    ///Creates a raw value instruction with the given `value` and `ty`
    pub fn raw(value: IRPointer<Operand>, ty: IRTypeId) -> Instruction {
        debug_assert!(value.len() == 1);
        Instruction {
            operands: value,
            instruction_type: InstructionType::RawValue,
            value_type: ty,
        }
    }
    ///Creates a call instruction that calls the function `func` with the arguments `args`. The provided `func_ret` is the return type of the function used as type of the instruction
    pub fn call(func: IRPointer<Context>, func_ret: IRTypeId, args: IRPointer<Operand>) -> Instruction {
        Instruction {
            operands: args,
            instruction_type: InstructionType::FunctionCall { func },
            value_type: func_ret,
        }
    }
}
