use smallvec::smallvec;

use crate::{
    Component, ComponentValueBuilder, Function, IRError, IRErrorDescription, IRErrorKind,
    IRPointer, IRStorage, IRType, IRTypeId, Instruction, Label, Opcode, Operand, SlynxIR,
    StyleProperty, Value,
};

// ── LabelBuilder (intermediate bookkeeping, dropped after generate()) ──────

/// Per-label state accumulated during body lowering.
///
/// These are **build-time only**; the finalized [`Label`] in `SlynxIR`
/// stores the final `instruction_start` / `instruction_count`.
struct LabelBuilder {
    ptr: IRPointer<Label, 1>,
    instruction_start: u32,
    instruction_count: u32,
}

// ── FunctionBuilder ────────────────────────────────────────────────────────

/// Builds a single function's IR by appending instructions to
/// `SlynxIR.instructions` **immediately** and returning [`Value`]
/// handles on the fly.
///
/// # Sealed-block discipline
///
/// Instructions are emitted into the **current block**.  Switching to
/// a different block seals the current one (no more instructions can
/// be appended to it), then opens the target.  This guarantees that
/// every block's instructions form a **contiguous** range in the
/// instruction stream, which is both cache-friendly and simplifies
/// label-range bookkeeping.
///
/// # SSA by construction
///
/// Each call to [`emit`](Self::emit) produces a fresh [`Value`] whose
/// numeric index equals the instruction's position in
/// `SlynxIR.instructions`.  Because an instruction is never mutated
/// after emission, every `Value` corresponds to exactly one defining
/// instruction — the definition of Static Single Assignment.
pub struct FunctionBuilder<'a> {
    ir: &'a mut SlynxIR,
    argument_values: Vec<Value>,
    func_id: IRPointer<Function, 1>,
    labels: Vec<LabelBuilder>,
    current_label_idx: usize,
    /// `true` after the first `switch_to_block` (i.e., a block is open).
    label_open: bool,
}

impl<'a> FunctionBuilder<'a> {
    pub fn new(func_id: IRPointer<Function, 1>, ir: &'a mut SlynxIR) -> Self {
        Self {
            ir,
            argument_values: Vec::new(),
            func_id,
            labels: Vec::new(),
            current_label_idx: 0,
            label_open: false,
        }
    }

    pub fn arguments(&self) -> &[Value] {
        &self.argument_values
    }

    ///Sets the function type to have the given `args` and return the given `ret` type, and returns the Values that represents the arguments of this function. Thus `out[0]` is the first argument, `out[1]` the second, and so on
    pub fn set_function_type(&mut self, args: Vec<IRTypeId>, ret: IRTypeId) -> &[Value] {
        let ty = self.ir.get(self.func_id).ty();

        let IRType::Function(func_id) = self.ir.get_type(ty) else {
            unreachable!()
        };
        let func_ty = self.ir.get_function_type_mut(func_id);
        func_ty.insert_arg_types(&args);
        func_ty.set_return_type(ret);
        self.emit_function_args(&args);
        &self.argument_values
    }
    /// Emit `Arg(i)` instructions for the function parameters and
    /// return the list of resulting values.
    ///
    /// Call exactly once, right after the entry label is entered.
    fn emit_function_args(&mut self, arg_types: &[IRTypeId]) {
        let can_emit = self.label_open;
        if !can_emit {
            self.label_open = true;
        }
        let values = arg_types
            .iter()
            .enumerate()
            .map(|(i, &ty)| self.emit(Opcode::Arg(i as u32), smallvec![], ty))
            .collect::<Vec<_>>();
        self.argument_values.extend_from_slice(&values);
        self.label_open = can_emit;
    }
    // ── IR access ────────────────────────────────────────────────────

    pub fn ir(&mut self) -> &mut SlynxIR {
        self.ir
    }

    // ── Label management ─────────────────────────────────────────────

    /// Allocate a new label and return its handle.  The label is *not*
    /// entered; call [`switch_to_block`](Self::switch_to_block) to start
    /// emitting instructions into it.
    pub fn create_label(&mut self, name: &str) -> IRPointer<Label, 1> {
        let label = self.ir.insert_label(self.func_id, name);
        self.labels.push(LabelBuilder {
            ptr: label,
            instruction_start: 0,
            instruction_count: 0,
        });
        label
    }

    /// Seal the current block and switch to `label`.
    ///
    /// Once a block is sealed, no further instructions can be emitted
    /// into it; you may not switch back.
    ///
    /// The first time a label is entered, `BlockParam` instructions are
    /// automatically emitted for each of the label's declared arguments.
    pub fn switch_to_block(&mut self, label: IRPointer<Label, 1>) -> Result<(), IRError> {
        // ── Seal existing block ──
        let curr_inst_count = self.ir.instructions.len() as u32;
        if self.label_open {
            let prev = &mut self.labels[self.current_label_idx];
            prev.instruction_count = curr_inst_count - prev.instruction_start;
        }

        let idx = self
            .labels
            .iter()
            .position(|lb| lb.ptr == label)
            .ok_or(IRError::new(
                IRErrorKind::InvalidLabelSwitch,
                crate::IRErrorDescription::InexistentLabel,
            ))?;

        // Sealed-block discipline: reject re-entry.
        if self.label_open && idx == self.current_label_idx {
            return Ok(()); // no-op switch to same block
        }
        // If instruction_start != 0 we've been entered before → sealed.
        if self.labels[idx].instruction_start != 0 {
            return Err(IRError::new(
                IRErrorKind::InvalidLabelSwitch,
                IRErrorDescription::SealedDescription,
            ));
        }

        // ── Open target label ──
        self.current_label_idx = idx;
        let lb = &mut self.labels[idx];
        lb.instruction_start = self.ir.instructions.len() as u32;
        self.label_open = true;

        // Emit BlockParam pseudo-instructions for the label's arguments.
        let arg_types: Vec<_> = self.ir.get(label).arguments().to_vec();
        let start = lb.instruction_start;
        for (i, &arg_ty) in arg_types.iter().enumerate() {
            let inst = Instruction::block_param(i as u32, arg_ty);
            self.ir.instructions.push(inst);
        }
        // Adjust instruction_start to include the BlockParam instructions.
        // These are part of this label's instruction range.
        lb.instruction_start = start;

        Ok(())
    }

    /// Convenience: alias for `switch_to_block`.
    pub fn goto(&mut self, label: IRPointer<Label, 1>) -> Result<(), IRError> {
        self.switch_to_block(label)
    }

    // ── Core emission ────────────────────────────────────────────────

    /// Emit an instruction and return its resulting [`Value`].
    ///
    /// The returned `Value` numerically equals the instruction's index
    /// in `SlynxIR.instructions`.
    #[inline]
    pub fn emit(
        &mut self,
        opcode: Opcode,
        operands: impl Into<smallvec::SmallVec<[Value; 4]>>,
        value_type: IRTypeId,
    ) -> Value {
        debug_assert!(
            self.label_open,
            "must switch to a block before emitting instructions"
        );
        let idx = self.ir.instructions.len() as u32;
        self.ir.instructions.push(Instruction {
            opcode,
            operands: operands.into(),
            value_type,
        });
        // Maintain label instruction count
        self.labels[self.current_label_idx].instruction_count += 1;
        Value::instruction(idx)
    }
    pub fn emit_void(
        &mut self,
        opcode: Opcode,
        operands: impl Into<smallvec::SmallVec<[Value; 4]>>,
    ) -> Value {
        debug_assert!(
            self.label_open,
            "must switch to a block before emitting instructions"
        );
        let idx = self.ir.instructions.len() as u32;
        self.ir.instructions.push(Instruction {
            opcode,
            operands: operands.into(),
            value_type: self.ir.void_type(),
        });
        // Maintain label instruction count
        self.labels[self.current_label_idx].instruction_count += 1;
        Value::instruction(idx)
    }

    /// Look up the result type of the instruction that produced `v`.
    pub fn value_type(&self, v: Value) -> IRTypeId {
        self.ir.instructions[v.idx()].value_type
    }
    pub fn component(
        &'a mut self,
        component: IRPointer<Component, 1>,
    ) -> ComponentValueBuilder<'a, 'a> {
        let ty = self.ir().get(component).ty;
        ComponentValueBuilder::new(self, ty)
    }

    // ── Convenience helpers ──────────────────────────────────────────

    /// Emit a constant (`Const`) instruction.
    pub fn emit_const(&mut self, operand: Operand, ty: IRTypeId) -> Value {
        self.emit(Opcode::Const(operand), smallvec![], ty)
    }

    /// Emit a BlockParam reference (used internally by `switch_to_block`).
    pub fn block_param(&self, label: IRPointer<Label, 1>, index: usize) -> Value {
        // Find the label builder to get the instruction_start
        let lb = self
            .labels
            .iter()
            .find(|lb| lb.ptr == label)
            .expect("label not found");
        Value::instruction(lb.instruction_start + index as u32)
    }

    // ── Finalisation ─────────────────────────────────────────────────

    /// Finalize the function: write label metadata and return the
    /// function pointer.
    pub fn generate(mut self) -> IRPointer<Function, 1> {
        // Seal the last block
        if self.label_open {
            let last = &mut self.labels[self.current_label_idx];
            last.instruction_count = self.ir.instructions.len() as u32 - last.instruction_start;
        }

        // Persist label instruction ranges onto the SlynxIR labels.
        for lb in &self.labels {
            let label = self.ir.get_mut(lb.ptr);
            label.instruction_start = lb.instruction_start;
            label.instruction_count = lb.instruction_count;
        }

        // Set the function's label pointer.
        if let Some(first) = self.labels.first() {
            self.ir
                .get_mut(self.func_id)
                .set_label_ptr(first.ptr.with_runtime_length(self.labels.len()));
        }
        self.func_id
    }
}

// ── Binary / arithmetic convenience methods ────────────────────────────────

macro_rules! impl_binop {
    ($($name:ident => $op:ident),+ $(,)?) => {
        impl FunctionBuilder<'_> {
            $(
                pub fn $name(&mut self, a: Value, b: Value) -> Value {
                    let ty = self.value_type(a);
                    self.emit(Opcode::$op, smallvec![a, b], ty)
                }
            )*
        }
    };
}

impl_binop! {
    add => Add,
    sub => Sub,
    mul => Mul,
    div => Div,
    cmp => Cmp,
    gt => Gt,
    gte => Gte,
    lt => Lt,
    lte => Lte,
    and => And,
    or => Or,
    xor => Xor,
    shl => Shl,
    shr => Shr,
    ashr => AShr,
}

// ── Manual helpers ────────────────────────────────────────────────────────

impl FunctionBuilder<'_> {
    pub fn call(
        &mut self,
        func: IRPointer<Function, 1>,
        args: &[Value],
        ret_ty: IRTypeId,
    ) -> Value {
        self.emit(Opcode::Call(func), args.to_vec(), ret_ty)
    }

    pub fn branch(&mut self, label: IRPointer<Label, 1>, args: &[Value]) -> Value {
        self.emit_void(Opcode::Br(label), args.to_vec())
    }

    pub fn branch_conditional(
        &mut self,
        condition: Value,
        then_label: IRPointer<Label, 1>,
        else_label: IRPointer<Label, 1>,
        then_args: &[Value],
        else_args: &[Value],
    ) -> Value {
        let mut ops = smallvec![condition];
        ops.extend_from_slice(then_args);
        ops.extend_from_slice(else_args);
        self.emit_void(
            Opcode::Cbr {
                then_label,
                else_label,
            },
            ops,
        )
    }

    pub fn ret(&mut self, value: Value) -> Value {
        let ty = self.value_type(value);
        self.emit(Opcode::Ret, smallvec![value], ty)
    }

    pub fn allocate(&mut self, ty: IRTypeId) -> Value {
        self.emit(Opcode::Allocate, smallvec![], ty)
    }

    pub fn write(&mut self, slot: Value, value: Value) -> Value {
        let ty = self.value_type(value);
        self.emit(Opcode::Write, smallvec![slot, value], ty)
    }

    pub fn read(&mut self, slot: Value) -> Value {
        let ty = self.value_type(slot);
        self.emit(Opcode::Read, smallvec![slot], ty)
    }

    pub fn get_field(&mut self, object: Value, index: u16) -> Value {
        let ty = self.value_type(object);
        let field_ty = self.ir.get_field_type(ty, index);
        self.emit(Opcode::GetField(index), smallvec![object], field_ty)
    }

    pub fn set_field(&mut self, object: Value, index: u16, value: Value) -> Value {
        self.emit_void(Opcode::SetField(index), smallvec![object, value])
    }

    pub fn struct_literal(&mut self, ty: IRTypeId, fields: &[Value]) -> Value {
        self.emit(Opcode::Struct, fields.to_vec(), ty)
    }

    pub fn sapply(&mut self, property_code: StyleProperty, operands: &[Value]) -> Value {
        self.emit_void(Opcode::SApply { property_code }, operands.to_vec())
    }

    pub fn initcall(&mut self, func: IRPointer<Function, 1>, operands: &[Value]) -> Value {
        self.emit_void(Opcode::InitCall(func), operands.to_vec())
    }
}
