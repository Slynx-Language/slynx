# Stylesheet System

This document describes how the stylesheet system works across the compiler pipeline and how to extend it with new style properties.

## Pipeline Overview

```
Source code
  │
  ▼ Parser (crates/parser/src/styles.rs)
  │   → ASTDeclarationKind::StyleSheet
  │
  ▼ HIR (crates/hir/src/implementation/)
  │   → HirDeclarationKind::StyleSheet
  │
  ▼ Checker (crates/checker/src/styles.rs)
  │   → validated + typed
  │
  ▼ IR (crates/slynx_ir/src/ir/)
  │   → struct type + constructor fn + apply fn
  │
  ▼ Backend
      → @sapply with numeric property codes
```

## What a stylesheet produces

Each `stylesheet` declaration generates **3 IR constructs**:

| Construct | Purpose |
|---|---|
| **Struct type** | Holds resolved property values as fields |
| **Constructor function** (`__init_<Name>`) | Evaluates expressions, builds the struct |
| **Apply function** (`<Name>`) | Takes (component, struct), emits `@sapply` per property |

A component using `style: Foo(args)` gets two `@initcall` instructions: one for
its own init function, one for the style's apply function.

---

## File Map

### Parser

| File | Role |
|---|---|
| `crates/parser/src/styles.rs` | Parses `stylesheet`, style blocks, `uses` clauses, property definitions |
| `crates/parser/src/ast/mod.rs` | AST types: `StyleBlock`, `StyleSheetStatement`, `ASTDeclarationKind::StyleSheet` |

### HIR — Model types

| File | Role |
|---|---|
| `crates/hir/src/model/declarations.rs` | `HirStyleUsage`, `HirDeclarationKind::StyleSheet` |
| `crates/hir/src/model/types.rs` | `HirType::Style { args }` — type representation of a stylesheet |
| `crates/hir/src/model/expression.rs` | `style: Option<HirStyleUsage>` on `Text`/`Div` components |
| `crates/hir/src/model/statements.rs` | `StylesDefinition`, `HirStyleBlockKind`, `HirStyleBlock`, `HirStyleStatement` |

### HIR — Implementation

| File | Role |
|---|---|
| `crates/hir/src/implementation/declarations.rs` | `hoist_stylesheet()`, `resolve_stylesheet()`, `resolve_style_usage()` |
| `crates/hir/src/implementation/statements.rs` | `resolve_style_type()` — maps property name → HIR type |
| `crates/hir/src/implementation/components.rs` | Resolves `style:` on Text/Div specialized components |
| `crates/hir/src/error.rs` | `InvalidStyleEvent`, `InvalidStyleDefinition` errors |

### Checker

| File | Role |
|---|---|
| `crates/checker/src/styles.rs` | `check_stylesheet()`, `check_style_usage()`, `check_style_statement()` |
| `crates/checker/src/expr.rs` | `resolve_specialized()` — resolves `style` on Text/Div |
| `crates/checker/src/defaults.rs` | `default_stylesheet()` — fallback type assignment |

### IR

| File | Role |
|---|---|
| `crates/slynx_ir/src/ir/model/styles.rs` | **`StyleProperty` enum** — central mapping: variant = one property |
| `crates/slynx_ir/src/ir/helper/styles.rs` | Collect, inherit, populate struct, constructor, apply function |
| `crates/slynx_ir/src/ir/helper/types.rs` | `insert_stylesheet_type_for()` — set up function arg types |
| `crates/slynx_ir/src/ir/components.rs` | `get_style_application()`, `initialize_component()` — style in component init |
| `crates/slynx_ir/src/ir/mod.rs` | Hoisting, pre-pass, dependency-ordered lowering |
| `crates/slynx_ir/src/ir/temp.rs` | `AuxiliaryStyle` data, temp state management |
| `crates/slynx_ir/STYLES_TABLE.md` | Hardcoded numeric codes for each property |

---

## How each phase processes styles

### Parser

`parse_stylesheet()` parses:
- Name + typed args: `stylesheet Fg(color: int)`
- Optional `uses` clause: `uses Parent(args)`
- Body with regular statements and `styles { default { ... } }` blocks

Style blocks are parsed by `parse_styles_statement()` → `parse_style_block()`,
which handles state names (`default`, `hover`) and named expressions
(`propertyName: expr`).

### HIR — Hoist

`hoist_stylesheet()` registers the stylesheet's type as `HirType::Style { args }`
in the module, making it referencable.

### HIR — Resolve

`resolve_stylesheet()`:
1. Enters scope, resolves args (variables + types)
2. Updates `HirType::Style` with concrete arg types
3. Resolves body statements via `resolve_stylesheet_statement()`
4. Resolves `styles` blocks via `resolve_stylesblock()` → `resolve_style_definitions()`
   - Each property name is mapped to a HIR type via `resolve_style_type()`
5. Resolves `uses` entries via `resolve_style_usage()` → `HirStyleUsage`

For component expressions, `try_resolve_specialized()` detects `Text`/`Div` and
resolves their `style:` field via `resolve_style_usage()`.

### Checker

`check_stylesheet()`:
- Validates `uses` arg count and unifies param types with parent arg types
- Unifies definition expression types with their declared property types

`resolve_specialized()` in `expr.rs` calls `resolve_style_usage()` to type-check
component `style:` references.

**Known gap:** `default_expr()` for Text and Div uses `..` and ignores the
`style` field, so unresolved `Infer` types in style usage params are not caught.

### IR — Hoist

Each `StyleSheet` gets:
- An empty struct type
- An empty constructor function (`__init_<Name>`)
- An empty apply function
- An `AuxiliaryStyle` entry tying them together

### IR — Pre-pass

`collect_style_properties()` extracts definitions from `Default` blocks.
`resolve_style_inheritance()` merges parent `uses` properties with own
properties (own overrides parent), sorted by STYLES_TABLE code order.

### IR — Dependency-ordered lowering

Parent stylesheets are lowered before children. For each:

1. **`populate_style_struct_fields()`** — adds fields to the struct
2. **`create_style_constructor()`** — emits init function:
   - Calls parent init functions for inherited properties
   - Evaluates own property expressions
   - Builds struct literal, returns it
3. **`create_style_apply_function()`** — emits apply function:
   - Args: (component, struct)
   - For each field: `getfield` + `@sapply(code, component, value)`

### IR — Component init

`initialize_component()` emits:
```
@initcall ComponentInit, #component
@initcall ApplyStyle, #component, __init_StyleName(args)
```

---

## How to add a new style property

Edit exactly **2 files**:

### Step 1: HIR type mapping

**File:** `crates/hir/src/implementation/statements.rs`
**Function:** `resolve_style_type()`

```rust
pub fn resolve_style_type(&mut self, name: &str, span: Span) -> Result<TypeId> {
    let ty = match name {
        "backgroundColor" | "foregroundColor" => self.int32_type(),
        "opacity" => self.float32_type(),        // ← add
        _ => {
            let name = self.modules.intern_name(name);
            return Err(HIRError::invalid_style_definition(name, span));
        }
    };
    Ok(ty)
}
```

Available HIR type constructors:

| Helper | Represents |
|---|---|
| `self.int32_type()` | i32 |
| `self.float32_type()` | f32 |
| `self.bool_type()` | bool |
| `self.str_type()` | string |
| `self.add_tuple_type(vec![t1, t2, ...])` | tuple |

### Step 2: IR enum variant

**File:** `crates/slynx_ir/src/ir/model/styles.rs`

Add the variant with the next STYLES_TABLE code and update all match arms:

```rust
#[repr(u16)]
pub enum StyleProperty {
    BackgroundColor = 0,
    ForegroundColor = 1,
    Opacity = 7,            // ← add
}

impl StyleProperty {
    pub fn from_name(name: &str) -> Self {
        match name {
            "backgroundColor" => Self::BackgroundColor,
            "foregroundColor" => Self::ForegroundColor,
            "opacity" => Self::Opacity,            // ← add
            _ => panic!("Property {name} should have been caught by HIR"),
        }
    }

    pub fn ir_type(self, types: &IRTypes) -> IRTypeId {
        match self {
            Self::BackgroundColor | Self::ForegroundColor => types.int_type(),
            Self::Opacity => types.float_type(),     // ← add
        }
    }
}

impl std::fmt::Display for StyleProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BackgroundColor => write!(f, "BACKGROUND_COLOR"),
            Self::ForegroundColor => write!(f, "FOREGROUND_COLOR"),
            Self::Opacity => write!(f, "OPACITY"),   // ← add
        }
    }
}
```

The `code()` method is `self as u16` — it returns the discriminant. Keep codes
in sync with `crates/slynx_ir/STYLES_TABLE.md`.

### That's it

Everything else is generic and needs no changes:

- Parser — any property name works (`name: expr` syntax is generic)
- HIR resolving — `resolve_style_definitions()` is generic
- Checker — `check_style_statement()` just unifies types
- IR struct/constructor/apply — all iterate resolved properties generically
- Component lowering — generic `@initcall ApplyStyle` emission

---

## Compound property types

For `padding`, `margin`, `size` (Vec4):

| Layer | Code |
|---|---|
| HIR `resolve_style_type` | `self.add_tuple_type(vec![f32, f32, f32, f32])` |
| IR `ir_type()` | `types.create_or_get_tuple(vec![f32, f32, f32, f32])` |

For `border`, `shadow` — define a custom struct type in both HIR and IR.

---

## Implementation status

| Code | Property | HIR type | IR type | Implemented |
|---|---|---|---|---|
| 0 | backgroundColor | int32 | I32 | ✅ |
| 1 | foregroundColor | int32 | I32 | ✅ |
| 2 | padding | — | — | ❌ |
| 3 | margin | — | — | ❌ |
| 4 | size | — | — | ❌ |
| 5 | fontSize | — | — | ❌ |
| 6 | fontWeight | — | — | ❌ |
| 7 | opacity | — | — | ❌ |
| 8 | border | — | — | ❌ |
| 9 | shadow | — | — | ❌ |

---

## Known gaps

- `default_expr()` in `crates/checker/src/expr.rs:563` ignores the `style` field
  on Text/Div specialized components
- `Hover` style blocks are parsed and stored in HIR but never lowered in IR
  (only `Default` blocks are processed by `collect_style_properties()`)
