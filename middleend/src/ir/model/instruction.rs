use common::SymbolPointer;

use crate::{IRTypeId, Label, ir::model::Context};

use super::IRPointer;

#[derive(Debug, Clone, Copy)]
///A logical pointer that points a instruction to its slot
pub struct SlotPointer {
    ///The instruction that generated the slot
    instruction: IRPointer<Instruction, 1>,
    ///The slot generated
    slot: IRPointer<Slot, 1>,
}

#[derive(Debug, Clone, Copy)]
///A value that represents something on a slot. A slot is something on memory, anywhere, so this is practically a pointer to some value. But it's better to be
///understood as a variable
pub struct Slot {
    ///The type of this slot
    pub(crate) ty: IRTypeId,
}

#[derive(Debug, Clone)]
pub enum IRSpecializedComponent {
    Text(IRPointer<Value, 1>),
    Div(IRPointer<Value>),
}

#[derive(Debug, Clone)]
///A value inside the IR. Can be a function arg, a label arg or the result of a instruction
pub enum Value {
    Void,
    StructLiteral(IRTypeId, IRPointer<Value>),
    Specliazed(IRPointer<IRSpecializedComponent, 1>),
    StructLiteral(IRTypeId, IRPointer<Value>),
    Raw(IRPointer<Operand, 1>),
    Instruction(IRPointer<Instruction, 1>),
    Slot(IRPointer<Slot, 1>),
    LabelArg(usize),
    FuncArg(usize),
}
#[derive(Debug, Clone)]
///An operand that is used on a instruction. Mainly it's values that are used on the instructions
pub enum Operand {
    Bool(bool),
    Int(i64),
    Float(f64),
    ///Index into the SlynxIR string pool's entries. The backend resolves this to a StrHandle {offset, len}.
    String(SymbolPointer),
}

#[derive(Debug, Clone)]
///An enum that represents the type of instruction
pub enum InstructionType {
    ///Variant used for raw values. Their actual value is their operand
    RawValue,
    Struct,
    Component,
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
    ///Variant used for binary AND. The type is determines by the `value_type` and the left and right hand side are the `operands`
    And,
    ///Variant used for binary OR. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Or,
    ///Variant used for binary RIGHT_SHIFT. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Shr,
    ///Variant used for binary Arithmetic RIGHT_SHIFT. The type is determines by the `value_type` and the left and right hand side are the `operands`
    AShr,
    ///Variant used for binary LEFT_SHIFT. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Shl,
    ///Variant used for binary XOR. The type is determines by the `value_type` and the left and right hand side are the `operands`
    Xor,
    ///A branch operation that executes the code from the provided `branch`. If the branch's got arguments the `operands` of this instruction are used as the label args
    Br(IRPointer<Label, 1>),
    ///Conditional branch. Takes a boolean condition and two target labels with their arguments
    Cbr {
        then_label: IRPointer<Label, 1>,
        else_label: IRPointer<Label, 1>,
        then_args: IRPointer<Value>,
        else_args: IRPointer<Value>,
    },
    Allocate(IRPointer<Slot, 1>),
    Write(IRPointer<Slot, 1>),
    Read,
    Reinterpret,
    GetField(usize),
    SetField(usize),
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

    ///Gets the `index`'th propertie of the given `value`
    pub fn getfield(index: usize, value: IRPointer<Value, 1>, ty: IRTypeId) -> Self {
        Self {
            operands: value.with_length(),
            instruction_type: InstructionType::GetField(index),
            value_type: ty,
        }
    }
    ///Sets on the first value of the target, the given `index`th property value to be the second value on `target`.
    ///# Example
    /// insert_value(obj)
    /// insert_value(5)
    /// insert_instruction(setfield(0, obj, value)); //will set obj.0 = value
    pub fn setfield(index: usize, target: IRPointer<Value, 2>, ty: IRTypeId) -> Self {
        Self {
            operands: target.with_length(),
            instruction_type: InstructionType::SetField(index),
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
    pub fn shr(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Shr,
            value_type: ty,
        }
    }
    pub fn ashr(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::AShr,
            value_type: ty,
        }
    }
    pub fn shl(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Shl,
            value_type: ty,
        }
    }

    pub fn xor(ty: IRTypeId, values: IRPointer<Value, 2>) -> Self {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::Xor,
            value_type: ty,
        }
    }
    pub fn and(ty: IRTypeId, values: IRPointer<Value, 2>) -> Instruction {
        Self {
            operands: values.with_length(),
            instruction_type: InstructionType::And,
            value_type: ty,
        }
    }
    pub fn or(ty: IRTypeId, value: IRPointer<Value, 2>) -> Self {
        Self {
            operands: value.with_length(),
            instruction_type: InstructionType::Or,
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

    pub fn allocate(slot: IRPointer<Slot, 1>, ty: IRTypeId) -> Self {
        Self {
            operands: IRPointer::null(),
            instruction_type: InstructionType::Allocate(slot),
            value_type: ty,
        }
    }
    pub fn write(ty: IRTypeId, slot: IRPointer<Slot, 1>, value: IRPointer<Value, 1>) -> Self {
        Self {
            operands: value.with_length(),
            instruction_type: InstructionType::Write(slot),
            value_type: ty,
        }
    }
    pub fn read(ty: IRTypeId, value: IRPointer<Value, 1>) -> Self {
        Self {
            operands: value.with_length(),
            instruction_type: InstructionType::Read,
            value_type: ty,
        }
    }

    pub fn struct_literal(ty: IRTypeId, value: IRPointer<Value>) -> Self {
        Self {
            operands: value,
            instruction_type: InstructionType::Struct,
            value_type: ty,
        }
    }
    pub fn component(ty: IRTypeId, value: IRPointer<Value>) -> Self {
        Self {
            operands: value,
            instruction_type: InstructionType::Component,
            value_type: ty,
        }
    }
}
