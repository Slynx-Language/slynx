use crate::{IRTypeId, Label, ir::model::Context};

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
    ///Variant used for binary add. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Add,
    ///Variant used for binary sub. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Sub,
    ///Variant used for binary mul. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Mul,
    ///Variant used for binary div. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Div,
    ///Variant used for binary cmp. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Cmp,
    ///Variant used for binary gt. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Gt,
    ///Variant used for binary gte. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Gte,
    ///Variant used for binary lt. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Lt,
    ///Variant used for binary lte. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Lte,
    ///A branch operation that executes the code from the provided `branch`. If the branch's got arguments the `operands` of this instruction are used as the label args
    Br(IRPointer<Label, 1>),
    ///Conditional branch. Takes a boolean condition and two target labels with their arguments
    Cbr {
        then_label: IRPointer<Label, 1>,
        else_label: IRPointer<Label, 1>,
        then_args: IRPointer<Value>,
        else_args: IRPointer<Value>,
    },
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
        Self {
            operands: args,
            instruction_type: InstructionType::FunctionCall(func),
            value_type: func_ret,
        }
    }
    pub fn ret(value: IRPointer<Value, 1>, ty: IRTypeId) -> Self {
        Self {
            operands: value.with_length(),
            instruction_type: InstructionType::Ret,
            value_type: ty,
        }
    }
    pub fn add(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Add,
            value_type: ty,
        }
    }
    pub fn sub(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Sub,
            value_type: ty,
        }
    }
    pub fn mul(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Mul,
            value_type: ty,
        }
    }

    pub fn div(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Div,
            value_type: ty,
        }
    }
    pub fn cmp(ty: IRTypeId, values: IRPointer<Value, 2>) -> Instruction {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Cmp,
            value_type: ty,
        }
    }
    pub fn gt(ty: IRTypeId, value: IRPointer<Value, 2>) -> Self {
        Self {
            operands: value.with_length(),
            instruction_type: InstructionType::Gt,
            value_type: ty,
        }
    }
    pub fn gte(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Gte,
            value_type: ty,
        }
    }
    pub fn lt(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Lt,
            value_type: ty,
        }
    }
    pub fn lte(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Lte,
            value_type: ty,
        }
    }
    pub fn br(label: IRPointer<Label, 1>, args: IRPointer<Value>, label_ret_ty: IRTypeId) -> Self {
        Self {
            operands: args,
            instruction_type: InstructionType::Br(label),
            value_type: label_ret_ty,
        }
    }
    pub fn cbr(
        condition: IRPointer<Value, 1>,
        then_label: IRPointer<Label, 1>,
        else_label: IRPointer<Label, 1>,
        then_args: IRPointer<Value>,
        else_args: IRPointer<Value>,
        value_ty: IRTypeId,
    ) -> Self {
        Self {
            operands: condition.with_length(),
            instruction_type: InstructionType::Cbr {
                then_label,
                else_label,
                then_args,
                else_args,
            },
            value_type: value_ty,
        }
    }
}
