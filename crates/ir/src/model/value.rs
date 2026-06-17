use crate::{IRTypeId, InitValue, SymbolPointer};

/// A lightweight handle representing a value in the IR.
///
/// # SSA invariant
///
/// Every `Value` is defined exactly once.  The numeric index is the
/// position of the defining [`Instruction`] in the function's flat
/// instruction stream — i.e. `Value(n)` is the result of
/// `function.instructions[n]`.
///
/// # Special ranges
///
/// | Range            | Kind                                   |
/// |------------------|----------------------------------------|
/// | `VOID`           | The canonical void / no-value sentinel |
/// | `1..`            | Instruction result index               |
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub u32);

impl Value {
    /// Canonical void / no-value sentinel.
    pub const VOID: Self = Self(0);

    /// Create a `Value` that refers to instruction at position `idx`.
    #[inline]
    pub fn instruction(idx: u32) -> Self {
        Self(idx)
    }

    /// Raw index into the owning function's instruction array.
    #[inline]
    pub fn idx(self) -> usize {
        self.0 as usize
    }

    /// Returns `true` when this is the void sentinel.
    #[inline]
    pub fn is_void(self) -> bool {
        self.0 == 0
    }
}

// ── Operand (compile-time constant payload) ────────────────────────────────

/// A compile-time constant value stored directly inside an [`Instruction`]
/// (e.g. for `Const` or `RawValue` opcodes).
#[derive(Debug, Clone)]
pub enum Operand {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(SymbolPointer),
}

impl From<bool> for Operand {
    fn from(value: bool) -> Self {
        Operand::Bool(value)
    }
}
impl From<i64> for Operand {
    fn from(value: i64) -> Self {
        Operand::Int(value)
    }
}
impl From<f64> for Operand {
    fn from(value: f64) -> Self {
        Operand::Float(value)
    }
}
impl From<SymbolPointer> for Operand {
    fn from(value: SymbolPointer) -> Self {
        Operand::String(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Slot;

#[derive(Debug, Clone, Copy)]
pub struct GlobalValue {
    pub(crate) initial_value: InitValue,
    pub(crate) name: SymbolPointer,
}

impl GlobalValue {
    pub fn name(&self) -> SymbolPointer {
        self.name
    }
    pub fn initial_value(&self) -> InitValue {
        self.initial_value
    }
}
