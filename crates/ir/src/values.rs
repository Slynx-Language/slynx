// This file is intentionally left empty.
//
// The old `insert_values`, `insert_value`, `insert_operands`, `insert_single_operand`,
// `create_literal` and `insert_literal` methods on `SlynxIR` have been **removed**.
//
// ── Rationale ──
//
// * Instruction operands are now stored **inline** via `SmallVec<[Value; 4]>`.
//   No separate `ir.values` / `ir.operands` storage exists.
// * Constants are emitted as `Opcode::Const(Operand)` instructions.
// * The builder's `emit()` method appends directly to `ir.instructions` and returns
//   a `Value` handle immediately.
//
// ── Migration ──
//
// Instead of:
//   let val = ir.insert_value(Value::new_raw(op_ptr, ty));
//   ir.insert_instruction(label, Instruction::raw(val, ty), false);
//
// Use:
//   builder.emit_const(Operand::Bool(true), ty);
