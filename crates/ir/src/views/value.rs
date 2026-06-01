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
