# IR TODO

## High Priority

### Stable Allocation Identity

Problem:
`Read` instruction returns a new `Value` handle instead of returning the slot itself, breaking slot identity tracking. When `y = read i32, x` and `write i32, y, 10`, the backend cannot tell that both refer to the same allocation without full data-flow analysis.

Code:
`crates/ir/src/model/instruction.rs` lines 261-269 (`read` constructor)

Impact:
- Backends must implement use-def chain analysis to track slot aliases
- Simple slot-to-register mapping is impossible
- Control flow slot merging requires sophisticated reconstruction

Suggested Direction:
Either (a) make `Read` produce the slot value with the fetched data, or (b) add explicit slot copy/alias instructions.

### Slot Phi Nodes for Control Flow

Problem:
When control flow converges, allocated slots written in different branches have no merging mechanism. A slot written in both branches of an if-statement has ambiguous value after the merge.

Code:
`crates/ir/src/cfg/mod.rs` - CFG only handles branch targets, no phi node support

Impact:
- Backends cannot correctly handle slots across control flow
- Programs with slots in conditionals will generate incorrect code

Suggested Direction:
Add `SlotPhi(Vec<(LabelId, Value)>)` instruction to merge slot values at control flow joins.

### Slot Liveness Information

Problem:
No way to determine when a slot is no longer needed. All slots must be conservatively allocated for the entire function.

Code:
`crates/ir/src/ir.rs` - No liveness tracking in SlynxIR

Impact:
- Cannot optimize slot reuse
- Cannot eliminate dead stores
- Memory usage is always maximal

Suggested Direction:
Add liveness analysis pass or store live ranges in slot metadata.

## Medium Priority

### Meaningless Write Return Value

Problem:
`Write` instruction has `value_type` set and returns a `Value` from the builder, but produces no useful value.

Code:
`crates/ir/src/model/instruction.rs` line 271-280, `crates/ir/src/builder/functions.rs` lines 364-367

Impact:
- Confusing API for backends
- Potential for misuse in generated code

Suggested Direction:
Either make `Write` void (set `value_type` to void) or clarify that the return is intentionally meaningless.

### Slot Metadata for Memory Model

Problem:
Slots have only type information, no size, alignment, or address space metadata.

Code:
`crates/ir/src/model/instruction.rs` - Allocate has only `value_type`

Impact:
- Backends cannot optimize for packed structs
- No custom alignment support
- Cannot distinguish stack vs heap slots

Suggested Direction:
Extend `Allocate` with optional size/alignment hints or add separate `Alloca` with metadata.

## Low Priority

### Consistent Value Naming

Problem:
Formatter treats `Allocate` specially with `$` prefix while other values use `%t{idx}`.

Code:
`crates/ir/src/visualize/formatter.rs` lines 269-276

Impact:
- Minor inconsistency in IR dumps
- No effect on compilation

Suggested Direction:
Unify naming convention in formatter output.

### Built-in Slot Analysis Passes

Problem:
Backends must reimplement common slot analysis patterns.

Impact:
- Code duplication across backends
- Potential for inconsistent analysis results

Suggested Direction:
Add utility methods to SlynxIR for:
- Finding all uses of a slot
- Computing slot live ranges
- Tracking slot ownership chains