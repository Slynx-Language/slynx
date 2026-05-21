# Component Property IR Lowering: `ComponentProperty` Value Kind

## The Problem

Before this change, compiling a component that references its own properties
inside child style attributes failed with:

```
IR internal error: variable 'color' is not recognized by the IR
```

**Example that failed:**

```
component Jorgin {
    prop color = 0xff00ff;
    Text {
        style: Fg(color),
        text: "Jorginho neguinho"
    }
}
```

The expression `color` in `Fg(color)` was lowered as an `Identifier` carrying a
`VariableId`, but the IR had no mapping from that `VariableId` to any value.
Component properties are **not** function-scoped variables — they are reactive
fields whose runtime values are set by the parent. The IR cannot evaluate them
to a constant, so `temp.get_variable(id)` returned `None` and the lowering
aborted with `IRError::UnrecognizedVariable`.

## The Solution: `ValueKind::ComponentProperty`

We introduced a new symbolic value kind `ComponentProperty(usize)` to the IR's
`Value` type, analogous to the existing `ComponentChild(usize)`.

### What it is

```
ValueKind::ComponentProperty(index)
```

A `ComponentProperty(index)` is a **symbolic handle** that represents "the
property at field index `index` of the enclosing component instance." It is
resolved to the actual runtime value by the backend — the IR simply records
the reference.

This is exactly the same pattern as `ComponentChild(index)` (`#tN`), which
represents "child at index N of the enclosing component instance." Both are
runtime-resolved handles, not compile-time constants.

### How it works end-to-end

#### HIR phase

1. When resolving a component's members (`resolve_component_defs` in
   `crates/hir/src/implementation/declarations.rs`), each `prop name` creates
   a variable via `create_variable(name, ty, span)`, which returns a `VariableId`.

2. This `VariableId` is **now stored** on the `ComponentMemberDeclaration::Property`
   struct as a new `variable_id: VariableId` field (`crates/hir/src/model/declarations.rs`).

3. Expressions inside the component body that reference the property (e.g.
   `color` in `Fg(color)`) are resolved as `HirExpression::Identifier(variable_id)`,
   where `variable_id` matches the one stored on the property.

#### IR phase

4. During `initialize_component` (`crates/slynx_ir/src/ir/components.rs`),
   after restoring the main context and before emitting style constructor calls
   (Phase 3), the IR iterates through the component's properties:

```rust
for prop in props {
    if let ComponentMemberDeclaration::Property {
        variable_id, index, ..
    } = prop
    {
        let prop_value = self.generate_component_property_value(*index, comp_ptr);
        let prop_ptr = self.insert_value(prop_value);
        temp.add_variable(*variable_id, prop_ptr);
    }
}
```

5. `generate_component_property_value(index, comp_ptr)` looks up the field type
   from the component's IR type and creates a `Value::new_component_property(index, ty)`.

6. This value is registered in `temp.variables` under the property's `VariableId`.

7. When `get_usage_args` later calls `get_value_for(Identifier(variable_id))`,
   `temp.get_variable(variable_id)` finds the `ComponentProperty` value and
   returns it. The style constructor call now correctly receives a runtime
   handle (`#p0`) instead of failing with an unrecognized variable.

### Why not a struct?

Components are currently modeled as a flat collection of properties + children,
not as a struct with named fields like the stylesheet system. The stylesheet
system uses a dedicated struct type for each stylesheet's data, which enables
`getfield`/`setfield` access. Components could theoretically be refactored to
use a similar struct-based approach, where:

```
component Jorgin {
    prop color = 0xff00ff;
    // becomes a struct like:
    // struct JorginProps { color: int }
    // and the init function takes JorginProps as an argument
}
```

This would allow property access via `getfield(comp_props, index)` inside
the init function. However, this would be a significant refactoring of the
component lowering pipeline. The `ComponentProperty` approach was chosen
because:

1. **Minimal change** — It follows the existing `ComponentChild` pattern and
   requires no restructuring of the component model.
2. **Consistent with children** — Children are already referenced symbolically
   via `ComponentChild`, so properties follow the same design.
3. **Runtime-resolved** — Both properties and children are runtime values that
   the backend resolves; the IR only needs to record the reference.
4. **Backward-compatible** — Existing code that does not reference properties
   in styles works unchanged.

If a struct-based refactoring is pursued in the future, `ComponentProperty`
values can be replaced with `getfield(comp_self_arg, index)` inside the
init function — both produce the same runtime behavior.

## Files Changed

| File | Change |
|------|--------|
| `crates/hir/src/model/declarations.rs` | Added `variable_id: VariableId` to `ComponentMemberDeclaration::Property`. Updated `new_property`. |
| `crates/hir/src/implementation/declarations.rs` | Capture `create_variable` result and pass to `new_property`. |
| `crates/slynx_ir/src/ir/model/value.rs` | Added `ComponentProperty(usize)` to `ValueKind`. Added `new_component_property`. |
| `crates/slynx_ir/src/ir/visualize/formatter.rs` | Display `ComponentProperty(index)` as `#p{index}`. |
| `crates/slynx_ir/src/ir/helper/ir_value.rs` | Added `generate_component_property_value`. |
| `crates/slynx_ir/src/ir/components.rs` | Register property VariableIds as `ComponentProperty` values before Phase 3. |

## Future Work

The documentation at `crates/slynx_ir/docs/refactor/update-component-fallback-handling.md`
discusses moving default-value substitution to the HIR layer. Currently, property
defaults are still evaluated and stored in `TempComponentData::default_properties`
by the IR. This should eventually be handled at the HIR level so that:

```
component C {
    prop value = 5;
}
C {}  // desugared to C { value: 5 } by the HIR
```

The `ComponentProperty` value works regardless of whether a default exists —
it always produces a runtime reference to the property field.
