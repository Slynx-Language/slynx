use crate::{IRTypeId, ir::model::Context};

use super::IRPointer;

#[derive(Debug, Clone)]
///A value inside the IR. Can be a function arg, a label arg or the result of a instruction
pub enum Value {
    Raw(IRPointer<Operand, 1>),
    Instruction(IRPointer<Instruction, 1>),
    LabelArg(usize),
    FuncArg(usize),
}
#[derive(Debug, Clone)]
///An operand that is used on a instruction. Mainly it's values that are used on the instructions
pub enum Operand {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone)]
///An enum that represents the type of instruction
pub enum InstructionType {
    ///Variant used for raw values. Their actual value is their operand
    RawValue,
    ///Variant used for function calls. The `func` field is the pointer to the function context
    FunctionCall(IRPointer<Context, 1>),
    ///Returns the operand
    Ret,
}

#[derive(Debug, Clone)]
///A instruction that determines the compiler something that it should execute
pub struct Instruction {
    pub operands: IRPointer<Value>,
    pub instruction_type: InstructionType,
    pub value_type: IRTypeId,
}

impl Instruction {
    ///Creates a raw value instruction with the given `value` and `ty`
    pub fn raw(value: IRPointer<Value, 1>, ty: IRTypeId) -> Instruction {
        Instruction {
            operands: value.with_length::<0>(),
            instruction_type: InstructionType::RawValue,
            value_type: ty,
        }
    }
    ///Creates a call instruction that calls the function `func` with the arguments `args`. The provided `func_ret` is the return type of the function used as type of the instruction
    pub fn call(
        func: IRPointer<Context, 1>,
        func_ret: IRTypeId,
        args: IRPointer<Value>,
    ) -> Instruction {
        Instruction {
            operands: args,
            instruction_type: InstructionType::FunctionCall(func),
            value_type: func_ret,
        }
    }
    pub fn ret(value: IRPointer<Value, 1>, ty: IRTypeId) -> Self {
        Instruction {
            operands: value.with_length(),
            instruction_type: InstructionType::Ret,
            value_type: ty,
        }
    }
}
