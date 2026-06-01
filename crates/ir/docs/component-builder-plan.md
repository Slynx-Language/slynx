# Component IR Builder — Architecture & Migration Plan

## 1. Problem Statement

The current IR has two concerns conflated under a single `Component` type:

| Concern | Type | Location |
|---|---|---|
| **Definition** (structural schema) | `IRComponent` | `types/components.rs` |
| **Expression** (runtime instance) | `Component` | `model/components.rs` |

The existing `ComponentBuilder` (`builder/component.rs`) blurs them: it receives an `IRPointer<Component, 1>` (an expression pointer) but mutates the definition via `get_component_type_mut`:

```rust
// builder/component.rs:40-48
pub fn generate(self) -> IRPointer<Component, 1> {
    let component = self.ir.get(self.component_id);
    let IRType::Component(ty) = self.ir.get_type(component.ty) else { ... };
    let ty = self.ir.get_component_type_mut(ty);   // direct definition mutation
    ty.fields.extend_from_slice(&self.fields);
    ty.children.extend_from_slice(&self.children);
    self.component_id
}
```

Outside the builder, codegen mutates definitions directly:

```rust
// codegen/src/components.rs:213
ir.get_component_type_mut(component_id).insert_field(ty);
```

This creates lifecycle confusion: a component's schema can change after expressions reference it, with no validation or immutability guarantees.

## 2. End-State Architecture

### Two-Phase Separation

```
┌─────────────────────────────────────────────────────────┐
│                   LOWERING PIPELINE                      │
│                                                          │
│  ┌─ Phase 1: Definition Construction ─────────────────┐ │
│  │                                                     │ │
│  │  ir.component_builder("MyComp")                     │ │
│  │    .add_field(field_type)                           │ │
│  │    .add_child(child_type)                           │ │
│  │    .build()                  → IRComponent (frozen)  │ │
│  │                                                     │ │
│  └─────────────────────────────────────────────────────┘ │
│                          │                                │
│                          ▼                                │
│  ┌─ Phase 2: Expression Construction ─────────────────┐ │
│  │                                                     │ │
│  │  ir.expression_builder(component_ty)                │ │
│  │    .field(value_expr)                               │ │
│  │    .child(child_value)                              │ │
│  │    .build()                  → Component (runtime)   │ │
│  │                                                     │ │
│  └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

**Key invariant:** After `build()`, definitions are read-only. Expressions reference frozen definitions via `ty`.

### Definition Builder (`types/component_builder.rs`)

```rust
// ── New file ──

/// Builder for constructing an immutable `IRComponent` definition.
///
/// Usage:
///   let comp_id = ir.component_builder("Button")
///       .add_field(ir.str_type())         // label
///       .add_field(ir.bool_type())        // enabled
///       .add_child(ir.generic_component_type())
///       .build();
pub struct ComponentDefBuilder {
    ir: *mut SlynxIR,
    component_id: IRComponentId,
    type_id: IRTypeId,
    fields: SmallVec<[IRTypeId; 16]>,
    children: SmallVec<[IRTypeId; 16]>,
    built: bool,
}
```

API:

| Method | Description |
|---|---|
| `add_field(ty)` → `Self` | Append a field type (consumes, chainable) |
| `add_child(ty)` → `Self` | Append a child type (consumes, chainable) |
| `build()` → `IRComponentId` | Freeze definition, write into `IRTypes.components` |

Integration point in `SlynxIR` (via `api.rs` or `types/mod.rs`):

```rust
impl SlynxIR {
    pub fn component_builder(&mut self, name: SymbolPointer) -> ComponentDefBuilder {
        let (type_id, component_id) = self.types.create_empty_component(name);
        ComponentDefBuilder::new(self as *mut SlynxIR, component_id, type_id, name)
    }
}
```

**Enforcement:** After `build()`:
- The builder panics on any further method call (`built: bool` guard)
- `IRComponent.fields`/`children` are written only once
- `get_component_type_mut` can be deprecated or restricted (see §5)

### Expression Builder (`model/component_expression_builder.rs`)

```rust
// ── New file ──

/// Builder for constructing a runtime `Component` expression.
///
/// Usage:
///   let comp_val = ir.expression_builder(button_type)
///       .field(label_value)     // matches 1st field of definition
///       .field(enabled_value)   // matches 2nd field of definition
///       .child(child_value)
///       .build();
pub struct ComponentExprBuilder<'a> {
    ir: &'a mut SlynxIR,
    component_ty: IRTypeId,
    field_values: Vec<Value>,
    child_values: Vec<Value>,
    built: bool,
}
```

API:

| Method | Description |
|---|---|
| `field(v)` → `Self` | Append a field value (consumes, chainable) |
| `child(v)` → `Self` | Append a child value (consumes, chainable) |
| `build()` → `Value` | Emit `Opcode::Component`, store in `SlynxIR.components` |

Integration in `SlynxIR`:

```rust
impl SlynxIR {
    pub fn expression_builder(&mut self, component_ty: IRTypeId) -> ComponentExprBuilder {
        ComponentExprBuilder::new(self, component_ty)
    }
}
```

**Validation in `build()`:**

```rust
pub fn build(self) -> Value {
    let def = self.ir.types.get_component_type(component_id);
    assert!(self.field_values.len() == def.fields.len(),
        "field count mismatch");
    assert!(self.child_values.len() == def.children.len(),
        "child count mismatch");
    // (future: type-check each value against the definition)
    let comp = Component::new(self.component_ty, ...);
    let ptr = self.ir.components.len();
    self.ir.components.push(comp);
    // emit Opcode::Component returning a Value
}
```

## 3. Current → Target State Mapping

### Definition Creation Sites

| Current code | Target code |
|---|---|
| `ir.get_component_type_mut(id).insert_field(ty)` | `ir.component_builder(name).add_field(ty).add_child(...).build()` |
| `ir.get_component_type_mut(id).insert_child(ty)` | (same builder, chained) |
| `codegen::components.rs::initialize_component` (direct `insert_field`) | Move to builder, hoist before expression phase |

### Expression Creation Sites

| Current code | Target code |
|---|---|
| `ctx.emit(Opcode::Component, ..., ty)` then `ctx.set_field(...)` | `ir.expression_builder(ty).field(v).child(v).build()` |
| `codegen::components.rs::get_component_expression` (manual field-by-field) | Replace with expression builder |

### Struct Builder Alignment

The existing `StructBuilder` (`builder/structs.rs`) follows the same pattern as the proposed definition builder and can be kept as-is. The new component definition builder mirrors its approach but uses a consuming builder pattern instead of `&mut self`.

## 4. Migration Strategy (5 Phases)

### Phase 1 — Add builders alongside existing code

- Create `types/component_builder.rs` with `ComponentDefBuilder`
- Create `model/component_expression_builder.rs` with `ComponentExprBuilder`
- Wire them into `SlynxIR` (new methods, no removals)
- Existing `builder/component.rs` `ComponentBuilder` remains untouched
- **No behavioural changes.** Existing `get_component_type_mut`, `insert_field`, `insert_child` still work.

### Phase 2 — Migrate definition construction

- Change `codegen::components.rs::initialize_component` to use `ir.component_builder()`
- Change any other definition-site code to use the builder
- Add doc-comment deprecation notices on `IRComponent::insert_field` / `insert_child`
- Add a `#[track_caller]` guard in `insert_field` that warns at runtime:

```rust
pub fn insert_field(&mut self, field: IRTypeId) {
    if self.frozen {
        panic!("IRComponent::insert_field called after freeze — use ComponentDefBuilder");
    }
    self.fields.push(field);
}
```

### Phase 3 — Migrate expression construction

- Change `codegen::components.rs::get_component_expression` to use `ir.expression_builder()`
- Remove `Opcode::Component` manual emission + `set_field` calls at expression sites
- Add validation in `ComponentExprBuilder::build()` (start with len checks, add type checks later)

### Phase 4 — Deprecate old API

- Mark `get_component_type_mut` as `#[deprecated]`
- Mark `insert_field` / `insert_child` as `#[deprecated]`
- Mark old `ComponentBuilder` as `#[deprecated]`
- Run `cargo build` with `#[deny(deprecated)]` to find all holdouts

### Phase 5 — Remove old API

- Remove `get_component_type_mut` (or make `pub(crate)` and limit to builder module)
- Remove `insert_field` / `insert_child` from `IRComponent`
- Remove old `builder/component.rs`
- Make `IRComponent.fields` / `children` read-only (remove `pub(crate)` on fields, keep getters)

## 5. Immutability Enforcement

### Definition Freezing

```rust
// types/components.rs
#[derive(Debug, Clone)]
pub struct IRComponent {
    pub(crate) name: SymbolPointer,
    fields: SmallVec<[IRTypeId; 16]>,       // was pub(crate), now private
    children: SmallVec<[IRTypeId; 16]>,      // same
    #[cfg(debug_assertions)]
    frozen: bool,
}
```

- `fields` / `children` become private after Phase 5
- Read access via `fields()` / `children()` getters only
- The builder is the **sole** mutation path

### Expression Freezing

```rust
// model/components.rs
#[derive(Debug, Clone)]
pub struct Component {
    ty: IRTypeId,
    values: IRPointer<Value>,
    ui_instruction: IRPointer<Instruction>,
}
```

- All fields already `pub(crate)`, but should become private post-migration
- Construction only via `ComponentExprBuilder` or `Component::new(...)` (the latter internal)
- No setter methods

## 6. Timeline / File Manifest

### New files

| File | Purpose |
|---|---|
| `types/component_builder.rs` | `ComponentDefBuilder` — builds frozen `IRComponent` |
| `model/component_expression_builder.rs` | `ComponentExprBuilder` — builds runtime `Component` |

### Modified files

| File | Phase | Change |
|---|---|---|
| `types/mod.rs` | 1 | Add `component_builder()` method on `IRTypes`/`SlynxIR` |
| `types/components.rs` | 2 | Add `frozen` flag, deprecation guard on `insert_field/insert_child` |
| `types/components.rs` | 5 | Make `fields`/`children` private |
| `model/components.rs` | 3 | Add `expression_builder()` integration |
| `model/components.rs` | 5 | Make fields private |
| `builder/component.rs` | 4 | Add `#[deprecated]` |
| `builder/component.rs` | 5 | Remove file |
| `codegen/src/components.rs` | 2-3 | Migrate to builders |
| `api.rs` | 1 | Add `component_builder()`, `expression_builder()` |

### Removed at end state

| Code | Replaced by |
|---|---|
| `IRComponent::insert_field` | `ComponentDefBuilder::add_field` |
| `IRComponent::insert_child` | `ComponentDefBuilder::add_child` |
| `IRTypes::get_component_type_mut` | Builder + read-only getter |
| `builder::ComponentBuilder` | `ComponentDefBuilder` + `ComponentExprBuilder` |

## 7. Risk Assessment

| Risk | Likelihood | Mitigation |
|---|---|---|
| Existing code bypasses builders | High (during migration) | Add `frozen` runtime guard, `#[deprecated]` attributes |
| Expression builder validation too strict | Medium | Start with len-only validation; add type checking behind a feature flag |
| Performance regression from builder allocs | Low | Builders are stack-local, `SmallVec` for fields |
| Double-`build()` panic in production | Low | `built` flag with `#[cfg(debug_assertions)]` only; can be made unconditional if desired |
