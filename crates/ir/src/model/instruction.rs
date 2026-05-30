use smallvec::SmallVec;

use crate::{Function, IRPointer, IRTypeId, Label, Operand, StyleProperty, Value};

// ── Opcode ─────────────────────────────────────────────────────────────────

/// Every instruction kind understood by the IR.
///
/// Opcodes that carry a payload (e.g. `Br`, `Call`) embed the relevant
/// IR references directly — no separate lookup table required.
#[derive(Debug, Clone)]
pub enum Opcode {
    // ═══════════════════════════════════════════════════════════════════
    //  Values
    // ═══════════════════════════════════════════════════════════════════
    /// A compile-time constant.  The sole operand (if present) is the
    /// [`Operand`] itself, stored inline.
    Const(Operand),

    /// The `n`-th function argument.  Produced during entry-block setup.
    Arg(u32),

    /// The `n`-th block (label) argument.
    BlockParam(u32),

    // ═══════════════════════════════════════════════════════════════════
    //  Memory
    // ═══════════════════════════════════════════════════════════════════
    /// Allocate a stack slot of the instruction's value-type.
    Allocate,

    /// Write a value into a previously allocated slot.
    /// Operands: `[slot, value]`.
    Write,

    /// Read from a slot.
    /// Operands: `[slot]`.
    Read,

    // ═══════════════════════════════════════════════════════════════════
    //  Arithmetic / Comparison / Bitwise
    // ═══════════════════════════════════════════════════════════════════
    Add,
    Sub,
    Mul,
    Div,
    Cmp,
    Gt,
    Gte,
    Lt,
    Lte,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    AShr,

    // ═══════════════════════════════════════════════════════════════════
    //  Aggregates & fields
    // ═══════════════════════════════════════════════════════════════════
    /// Construct a struct literal.  Operands are the field values.
    Struct,

    /// Construct a component.  Operands are the component field values.
    Component,

    /// Extract field `index` from a struct/component.
    /// Operands: `[object]`.
    GetField(u16),

    /// Set field `index` on a struct/component.
    /// Operands: `[object, value]`.
    SetField(u16),

    // ═══════════════════════════════════════════════════════════════════
    //  Control flow
    // ═══════════════════════════════════════════════════════════════════
    /// Unconditional branch to a label.
    /// Operands are the label's block arguments.
    Br(IRPointer<Label, 1>),

    /// Conditional branch.
    /// Operands: `[condition]`.
    Cbr {
        then_label: IRPointer<Label, 1>,
        else_label: IRPointer<Label, 1>,
    },

    /// Return from the current function.
    /// Operands: `[return_value]`.
    Ret,

    /// Call a function.  Operands are the call arguments.
    Call(IRPointer<Function, 1>),

    // ═══════════════════════════════════════════════════════════════════
    //  Reinterpret / raw
    // ═══════════════════════════════════════════════════════════════════
    /// Reinterpret the bits of a value as a different type.
    Reinterpret,

    /// A "raw value" wrapper.  The sole operand is a [`Value`] that
    /// holds the raw bits.
    RawValue,

    // ═══════════════════════════════════════════════════════════════════
    //  UI (framework-specific)
    // ═══════════════════════════════════════════════════════════════════
    /// Apply a style property to a UI component.
    SApply {
        property_code: StyleProperty,
    },

    /// Call a style initializer function on a component.
    InitCall(IRPointer<Function, 1>),
}

// ── Instruction ────────────────────────────────────────────────────────────

/// A single instruction in the IR.
///
/// # Memory layout
///
/// The struct keeps `operands` inline via a [`SmallVec`] so that
/// common 0–4 operand instructions never heap-allocate.  Call and
/// struct-literal instructions with many operands still pay only a
/// single dynamic allocation for the operand vector.
///
/// # SSA
///
/// The result of an instruction is **implicit**: `Value(n)` is the
/// result of `instructions[n]`.  No explicit result field is stored.
#[derive(Debug, Clone)]
pub struct Instruction {
    /// The operation this instruction performs.
    pub opcode: Opcode,

    /// SSA operand handles.  The length and interpretation depend on
    /// `opcode` (see the variant-level documentation).
    pub operands: SmallVec<[Value; 4]>,

    /// The type this instruction produces.
    pub value_type: IRTypeId,
}

// ── Convenience constructors ───────────────────────────────────────────────

macro_rules! binop_ctor {
    ($name:ident, $variant:ident) => {
        pub fn $name(ty: IRTypeId, operands: SmallVec<[Value; 4]>) -> Instruction {
            Instruction {
                opcode: Opcode::$variant,
                operands,
                value_type: ty,
            }
        }
    };
}

impl Instruction {
    /// Build a call instruction.
    pub fn call(
        func: IRPointer<Function, 1>,
        args: SmallVec<[Value; 4]>,
        ret_ty: IRTypeId,
    ) -> Self {
        Instruction {
            opcode: Opcode::Call(func),
            operands: args,
            value_type: ret_ty,
        }
    }

    /// Build an unconditional branch.
    pub fn br(label: IRPointer<Label, 1>, args: SmallVec<[Value; 4]>, ty: IRTypeId) -> Self {
        Instruction {
            opcode: Opcode::Br(label),
            operands: args,
            value_type: ty,
        }
    }

    /// Build a conditional branch.
    pub fn cbr(
        cond: Value,
        then_label: IRPointer<Label, 1>,
        else_label: IRPointer<Label, 1>,
        then_args: SmallVec<[Value; 4]>,
        else_args: SmallVec<[Value; 4]>,
        ty: IRTypeId,
    ) -> Self {
        let mut operands = SmallVec::with_capacity(1 + then_args.len() + else_args.len());
        operands.push(cond);
        operands.extend(then_args);
        operands.extend(else_args);
        Instruction {
            opcode: Opcode::Cbr {
                then_label,
                else_label,
            },
            operands,
            value_type: ty,
        }
    }

    pub fn const_value(op: Operand, ty: IRTypeId) -> Self {
        Instruction {
            opcode: Opcode::Const(op),
            operands: SmallVec::new(),
            value_type: ty,
        }
    }

    pub fn arg(index: u32, ty: IRTypeId) -> Self {
        Instruction {
            opcode: Opcode::Arg(index),
            operands: SmallVec::new(),
            value_type: ty,
        }
    }

    pub fn block_param(index: u32, ty: IRTypeId) -> Self {
        Instruction {
            opcode: Opcode::BlockParam(index),
            operands: SmallVec::new(),
            value_type: ty,
        }
    }

    // ── Standard constructors kept for backward compat ──

    pub fn raw(ty: IRTypeId, value: Value) -> Self {
        let mut operands = SmallVec::new();
        operands.push(value);
        Instruction {
            opcode: Opcode::RawValue,
            operands,
            value_type: ty,
        }
    }

    pub fn ret(ty: IRTypeId, value: Value) -> Self {
        let mut operands = SmallVec::new();
        operands.push(value);
        Instruction {
            opcode: Opcode::Ret,
            operands,
            value_type: ty,
        }
    }

    pub fn read(ty: IRTypeId, slot: Value) -> Self {
        let mut operands = SmallVec::new();
        operands.push(slot);
        Instruction {
            opcode: Opcode::Read,
            operands,
            value_type: ty,
        }
    }

    pub fn write(ty: IRTypeId, slot: Value, value: Value) -> Self {
        let mut operands = SmallVec::new();
        operands.push(slot);
        operands.push(value);
        Instruction {
            opcode: Opcode::Write,
            operands,
            value_type: ty,
        }
    }

    pub fn getfield(index: u16, object: Value, ty: IRTypeId) -> Self {
        let mut operands = SmallVec::new();
        operands.push(object);
        Instruction {
            opcode: Opcode::GetField(index),
            operands,
            value_type: ty,
        }
    }

    pub fn setfield(index: u16, object: Value, value: Value, ty: IRTypeId) -> Self {
        let mut operands = SmallVec::new();
        operands.push(object);
        operands.push(value);
        Instruction {
            opcode: Opcode::SetField(index),
            operands,
            value_type: ty,
        }
    }

    pub fn allocate(ty: IRTypeId) -> Self {
        Instruction {
            opcode: Opcode::Allocate,
            operands: SmallVec::new(),
            value_type: ty,
        }
    }

    pub fn struct_literal(ty: IRTypeId, fields: SmallVec<[Value; 4]>) -> Self {
        Instruction {
            opcode: Opcode::Struct,
            operands: fields,
            value_type: ty,
        }
    }

    pub fn component(ty: IRTypeId, fields: SmallVec<[Value; 4]>) -> Self {
        Instruction {
            opcode: Opcode::Component,
            operands: fields,
            value_type: ty,
        }
    }

    pub fn sapply(
        property_code: StyleProperty,
        operands: SmallVec<[Value; 4]>,
        ty: IRTypeId,
    ) -> Self {
        Instruction {
            opcode: Opcode::SApply { property_code },
            operands,
            value_type: ty,
        }
    }

    pub fn initcall(
        func: IRPointer<Function, 1>,
        operands: SmallVec<[Value; 4]>,
        ty: IRTypeId,
    ) -> Self {
        Instruction {
            opcode: Opcode::InitCall(func),
            operands,
            value_type: ty,
        }
    }

    // ── Binary op constructors ──

    binop_ctor!(add, Add);
    binop_ctor!(sub, Sub);
    binop_ctor!(mul, Mul);
    binop_ctor!(div, Div);
    binop_ctor!(cmp, Cmp);
    binop_ctor!(gt, Gt);
    binop_ctor!(gte, Gte);
    binop_ctor!(lt, Lt);
    binop_ctor!(lte, Lte);
    binop_ctor!(and, And);
    binop_ctor!(or, Or);
    binop_ctor!(xor, Xor);
    binop_ctor!(shl, Shl);
    binop_ctor!(shr, Shr);
    binop_ctor!(ashr, AShr);
}
