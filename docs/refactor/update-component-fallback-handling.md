# Move Default Value Handling from IR to HIR

## Problem

When a component property has a default value (e.g. `prop color = 0xff00ff`), the
IR currently evaluates and stores it in `TempComponentData::default_properties` via
`initialize_component` (Phase 1 in `crates/slynx_ir/src/ir/components.rs:309–317`).

This is wrong because:

1. **Properties are reactive**, not plain variables. Their default value is a
   compile-time fallback — the runtime value may differ when a parent provides it.
2. **Variable semantics don't apply.** A `prop` is not a function-scoped variable;
   it's a component field with potential reactivity. Treating it as a temp variable
   at the IR level conflates two separate concepts.
3. **The desugaring belongs earlier.** `C {}` should be semantically equivalent to
   `C { value: default }` at the HIR level, before IR lowering ever sees it.

## Current (broken) behavior

```
component Jorgin {
    prop color = 0xff00ff;
    Text {
        style: Fg(color),
        text: "Jorginho neguinho"
    }
}
```

1. **HIR** creates a `VariableId` for `color` via `create_variable` in
   `resolve_component_defs` (`crates/hir/src/implementation/declarations.rs:271`).
   References to `color` in children (e.g. `Fg(color)`) become
   `HirExpression::Identifier(color_variable_id)`.

2. **IR** (`initialize_component`) evaluates the default value expression and
   stores the result in `default_properties`, but **never registers** the
   `VariableId` → `IRPointer<Value>` mapping in `temp.variables`. When
   `get_usage_args` (line 411) tries to resolve `color`, the lookup fails with
   `IRError::UnrecognizedVariable`.

## Desired solution

**Default-value fallback should be resolved at the HIR level.** The HIR should
desugar property references inside component bodies before IR lowering.

### How it should work

1. In `resolve_component_defs` (or a follow-up pass), after collecting all
   `ComponentMemberDeclaration`s, walk the children's expression trees and
   replace any `Identifier` that refers to a component property with the
   property's default value expression (when one exists).

   Example before HIR output:
   ```
   component Jorgin {
       prop color = 0xff00ff;
       Text { style: Fg(color), text: "Jorginho neguinho" }
   }
   ```
   After desugaring:
   ```
   component Jorgin {
       prop color = 0xff00ff;
       Text { style: Fg(0xff00ff), text: "Jorginho neguinho" }
   }
   ```

2. `C {}` (no value provided) should be desugared to `C { value: default }`
   at the call site during HIR resolution — *not* handled later by the IR.

### What to change

| File | Change |
|------|--------|
| `crates/hir/src/implementation/declarations.rs` | In `resolve_component_defs`, after creating all members, walk children's expressions and substitute property references with their default values. OR create a separate desugaring pass. |
| `crates/hir/src/implementation/components.rs` | In `resolve_component_members` (call sites like `C {}`), if a required property is missing and has a default, inject the default expression. |
| `crates/slynx_ir/src/ir/components.rs` | Remove `default_properties` handling from Phase 1 of `initialize_component`. Properties with defaults are already desugared by the HIR — no special IR treatment needed. |
| `crates/slynx_ir/src/ir/temp.rs` | Remove `TempComponentData::default_properties` (no longer needed). |

### Benefits

- **Clear separation of concerns**: HIR handles semantic desugaring; IR lowers
  what it receives without special-casing property defaults.
- **Reactivity-friendly**: Properties remain a pure HIR concept; the IR never
  needs to know about default values.
- **Fixes the bug**: `color` in `Fg(color)` is replaced with `0xff00ff` before
  the IR sees it, so `get_value_for` never encounters an unrecognized variable.
- **Simplifies IR lowering**: `initialize_component` no longer needs Phase 1
  property-value evaluation or `default_properties` storage.
