use std::ops::{Deref, DerefMut};

use common::SymbolsModule;

use crate::{
    Component, Function, GlobalValue, IRBatchViewer, IRPointer, IRStorage, IRTypes, Instruction,
    Label, Opcode, Value,
};

/// The top-level IR store.
///
/// # Storage model
///
/// Everything is stored in flat, append-only `Vec`s:
///
/// * `instructions` — every instruction across all functions, in
///   emission order.  A [`Value`] handle is simply an index into this
///   array.
/// * `labels` — every block label across all functions.
/// * `functions` — every function.
///
/// There is **no** `instruction_pointers`, `values`, `operands`, or
/// `slots` indirection layer.  Instruction operands are stored inline
/// via [`SmallVec`](smallvec::SmallVec).
///
/// # DOD / cache-friendliness
///
/// The flat layout means that iterating over a function's instructions
/// is a linear scan of a dense `Vec<Instruction>`.  There are no
/// pointer-chasing `instruction_pointers` lookups.
#[derive(Debug)]
pub struct SlynxIR {
    ///Global values
    pub(crate) globals: Vec<GlobalValue>,
    /// All functions.
    pub(crate) functions: Vec<Function>,
    /// All UI components.
    pub(crate) components: Vec<Component>,
    /// All block labels across all functions.
    pub(crate) labels: Vec<Label>,
    /// **All** instructions, appended during lowering.
    pub(crate) instructions: Vec<Instruction>,
    /// **All** instructions that are impured appended during lowering.
    pub(crate) impure_instructions: Vec<Value>,
    /// Type storage.
    pub(crate) types: IRTypes,
    /// Interned string pool.
    pub strings: SymbolsModule<SlynxIR>,
}

impl Default for SlynxIR {
    fn default() -> Self {
        Self::new()
    }
}

impl SlynxIR {
    pub fn new() -> Self {
        let types = IRTypes::new();
        let void_ty = types.void_type();
        Self {
            globals: Vec::new(),
            functions: Vec::new(),
            components: Vec::new(),
            labels: Vec::new(),
            // Reserve index 0 with a dummy RawValue so that Value(0) never
            // collides with the result of a real instruction (e.g. Arg(0)).
            instructions: vec![Instruction {
                opcode: Opcode::RawValue,
                operands: smallvec::SmallVec::new(),
                value_type: void_ty,
            }],
            impure_instructions: Vec::new(),
            types,
            strings: SymbolsModule::new(),
        }
    }

    // ── Label creation ──────────────────────────────────────────────

    /// Create a new label with the given `name` and record it as
    /// belonging to `func`.  Returns the label's pointer.
    ///
    /// The label's `instruction_start` / `instruction_count` are
    /// filled in later by [`FunctionBuilder::generate`].
    pub(crate) fn insert_label(
        &mut self,
        func: IRPointer<Function, 1>,
        label_name: &str,
    ) -> IRPointer<Label, 1> {
        // Update the function's label count.
        self.functions[func.ptr()].insert_label();

        let name = self.strings.intern(label_name);
        let ptr = self.labels.len();
        self.labels.push(Label::new(name));
        IRPointer::new(ptr, 1)
    }

    // ── Value type queries ──────────────────────────────────────────

    /// Look up the result type of the instruction that produced `v`.
    pub fn value_type(&self, v: Value) -> crate::IRTypeId {
        self.instructions[v.idx()].value_type
    }

    // ── Access by pointer ───────────────────────────────────────────

    pub fn get_instruction(&self, v: Value) -> &Instruction {
        &self.instructions[v.idx()]
    }

    // ── Formatting ──────────────────────────────────────────────────

    /// Produce a textual dump of the IR (SIR format).
    pub fn format_sir(&self) -> String {
        let fmt = crate::Formatter::new(self);
        let mut out = fmt.format_types();
        out.push_str(&fmt.format_functions());
        out
    }
    pub fn view_all<'a, T>(&'a self) -> IRBatchViewer<'a, T>
    where
        Self: IRStorage<T>,
    {
        IRBatchViewer {
            ptr: IRPointer::new(0, self.get_storage().len()),
            ir: self,
        }
    }
}
impl Deref for SlynxIR {
    type Target = IRTypes;
    fn deref(&self) -> &Self::Target {
        &self.types
    }
}
impl DerefMut for SlynxIR {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.types
    }
}
