use smallvec::SmallVec;

use crate::{IRTypeId, SymbolPointer, Value};

/// A named block (basic block) in the IR.
///
/// # Instruction range
///
/// A label owns a **contiguous** range of the enclosing function's
/// flat instruction array.  The range is given by
/// `instruction_start .. instruction_start + instruction_count`.
/// Because the builder uses a **seal-on-switch** discipline (you
/// cannot append to a block after switching away from it), every
/// label's instructions are guaranteed contiguous and known at
/// finalization time.
///
/// Block arguments (`arguments`) act as SSA phi-nodes: values
/// flowing into this label from its predecessors.
#[derive(Debug, Clone)]
pub struct Label {
    name: SymbolPointer,

    /// Global index into the owner-function's `instructions` array.
    pub(crate) instruction_start: u32,

    /// Number of instructions in this block.
    pub(crate) instruction_count: u32,

    /// Types of this label's block (SSA phi) arguments.
    arguments: SmallVec<[IRTypeId; 2]>,
}

impl Label {
    pub fn new(name: SymbolPointer) -> Self {
        Self {
            name,
            instruction_start: 0,
            instruction_count: 0,
            arguments: SmallVec::new(),
        }
    }

    // ── Argument helpers ──

    pub fn insert_arguments(&mut self, types: &[IRTypeId]) {
        self.arguments.extend_from_slice(types);
    }

    pub fn arguments(&self) -> &[IRTypeId] {
        &self.arguments
    }

    pub fn add_argument(&mut self, ty: IRTypeId) {
        self.arguments.push(ty);
    }

    /// Return a `Value` handle representing the `index`-th block
    /// argument of this label.
    pub fn get_argument_value(&self, index: usize) -> Value {
        debug_assert!(index < self.arguments.len());
        // Block-param values are addressed by (label_ptr, index).
        // We encode them as instruction-index-base values beyond
        // the real instruction range.  The emitter/finalizer will
        // resolve them.
        let label_base = self.instruction_start + self.instruction_count;
        Value::instruction(label_base + index as u32)
    }

    // ── Instruction range ──

    pub fn instruction_start(&self) -> usize {
        self.instruction_start as usize
    }

    pub fn instruction_count(&self) -> usize {
        self.instruction_count as usize
    }

    /// Range `start .. end` into the enclosing function's instructions.
    pub fn instruction_range(&self) -> std::ops::Range<usize> {
        self.instruction_start as usize..(self.instruction_start + self.instruction_count) as usize
    }

    // ── Name ──

    pub fn name(&self) -> SymbolPointer {
        self.name
    }
}
