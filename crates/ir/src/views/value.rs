// This file is intentionally left empty.
//
// The old `IRViewer<Operand>` and `IRViewer<Value>` implementations
// have been **removed** because:
//
// * `Value` is now `#[repr(transparent)] pub struct Value(pub u32)` —
//   a lightweight handle, not a struct with `ValueKind`.
// * `Operand` constants are stored inline inside `Opcode::Const(Operand)`,
//   not in a separate `ir.operands` array.
//
// Value-type queries should use `SlynxIR::value_type(v)` instead.

use crate::{GlobalValue, IRTypeId, IRViewer, InitValue};

impl<'a> IRViewer<'a, GlobalValue> {
    pub fn ty(&self) -> IRTypeId {
        match self.initial_value {
            InitValue::ZeroInit(t) => t,
            InitValue::Constant(v) => self.ir.get_instruction(v).value_type,
            InitValue::Lazy(f) => self.ir.get_view(f.with_length()).get_return_type(),
        }
    }
    pub fn raw_name(&self) -> &str {
        self.ir.get_name(self.name)
    }
}
