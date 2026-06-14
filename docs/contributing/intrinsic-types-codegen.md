# Intrinsic Types in Codegen

This document describes how intrinsic types (`@intrinsic`-tagged objects like `Pixel`, `Rgba`, etc.) are lowered by the codegen, and the strategy for converting from HIR wrapper types to raw primitives that `sapply` instructions can work with.

## Why not pass structs through sapply

Intrinsic wrappers exist **only for type-checking** — they let the compiler validate that a `Pixel` is not passed where a `Color` is expected. At runtime they have no meaning. Passing wrapper structs through `sapply` would force every backend to know about every intrinsic type and manually unwrap them, making SIMD, animations, reactivity, and transitions more complex.

## Strategy: Lower wrappers to raw primitives in the constructor

The type checker validates against `expected_type` (the wrapper), but codegen extracts the inner raw value **before** storing it in the stylesheet struct field. The extraction happens in `create_style_constructor` (`crates/codegen/src/helper/styles.rs`), after `lower_expression` returns an IR value.

```
Type checker:  expected_type = Pixel  →  value type = Pixel  →  PASS
Codegen:       lower_expression → Pixel struct  →  extract .value → int
               store int in stylesheet struct field
               sapply(PADDING, [comp, int])
```

## Single-field intrinsics (e.g., Pixel)

`Pixel { value: int }` has exactly one field. The extraction is mechanical:

1. `lower_expression` returns the `Pixel` struct IR value
2. `ctx.get_field(value, 0)` extracts the inner `int`
3. The raw `int` is stored in the struct field
4. `sapply` receives `int`

### Where the extraction goes

In `create_style_constructor`, after lowering each property expression:

```rust
// After lower_expression returns the IR value
let value = ctx.lower_expression(&def.expression, hir, ir, types)?;

// If the expected type is a single-field intrinsic wrapper, extract the inner value
let value = match &*hir.get_type(&def.expected_type) {
    HirType::Struct { fields } if fields.len() == 1 => {
        ctx.get_field(value, 0)
    }
    _ => value,
};
field_values.push(value);
```

**No changes needed to:** `StyleProperty::ir_type()`, `populate_style_struct_fields`, `create_style_apply_function`, the IR opcode, or any backend. `ir_type()` continues to return `int` for `Padding`, the struct field stores `int`, and `sapply` passes `int`.

## Multi-field intrinsics: Rgba

`Rgba { r: int, g: int, b: int, a: int }` has four fields. There are several strategies depending on the target property.

### Strategy 1: Pack into a single primitive (for backgroundColor/foregroundColor)

Background color is already `int32` — the convention is `0xRRGGBBAA` packed into a single i32. In `create_style_constructor`:

```rust
// Rgba struct → packed int
let value = ctx.lower_expression(&def.expression, hir, ir, types)?;
let r = ctx.get_field(value, 0);
let g = ctx.get_field(value, 1);
let b = ctx.get_field(value, 2);
let a = ctx.get_field(value, 3);
// Pack: (r << 24) | (g << 16) | (b << 8) | a
let packed = ctx.or(ctx.shl(r, 24), ctx.or(ctx.shl(g, 16), ctx.or(ctx.shl(b, 8), a)));
field_values.push(packed);
```

This keeps `ir_type()` returning `int` for `BackgroundColor`/`ForegroundColor`.

### Strategy 2: Keep as tuple (for compound CSS values)

For properties where the raw components map directly to CSS (e.g., `border: 1px solid black` represented as `Pixel + Rgba`), store as a tuple of primitives.

### Strategy 3: Custom lowering via property-specific hooks

If packing/unpacking logic is complex or varies per property, add a per-property lowering hook on `StyleProperty`:

```rust
impl StyleProperty {
    pub fn lower_raw(&self, ctx: &mut Builder, struct_value: Value, hir: &SlynxHir) -> Value {
        match self {
            Self::BackgroundColor | Self::ForegroundColor => {
                // Rgba → packed int (Strategy 1)
                let r = ctx.get_field(struct_value, 0);
                let g = ctx.get_field(struct_value, 1);
                let b = ctx.get_field(struct_value, 2);
                let a = ctx.get_field(struct_value, 3);
                ctx.or(ctx.shl(r, 24), ctx.or(ctx.shl(g, 16), ctx.or(ctx.shl(b, 8), a)))
            }
            Self::Padding | Self::Margin => {
                // Single-field extraction
                ctx.get_field(struct_value, 0)
            }
            _ => struct_value, // no extraction needed
        }
    }
}
```

Called from `create_style_constructor` after `lower_expression`:

```rust
let mut value = ctx.lower_expression(&def.expression, hir, ir, types)?;
if let HirType::Struct { .. } = &*hir.get_type(&def.expected_type) {
    value = rp.property.lower_raw(ctx, value, hir);
}
field_values.push(value);
```

## What stays the same

| Component | Behavior |
|---|---|
| `resolve_style_type` (HIR) | Returns `expected_type` from `LangItems` lookup (e.g., `Pixel` type) |
| Type checker | Unifies expression type against `expected_type` — unchanged |
| `StyleProperty::ir_type()` | Returns raw primitive — unchanged |
| `populate_style_struct_fields` | Uses `ir_type()` — field type stays primitive |
| `create_style_apply_function` | Iterates fields, emits `sapply` — unchanged |
| `sapply` opcode | Receives primitive — unchanged |
| IR → Backend | Receives primitive — no knowledge of intrinsics |

## What changes

| File | Change |
|---|---|
| `crates/codegen/src/helper/styles.rs` | `create_style_constructor`: extract inner value after `lower_expression` |
| `crates/ir/src/model/styles.rs` | Optional: add `lower_raw()` method to `StyleProperty` for complex packing |
| `crates/hir/src/implementation/statements.rs` | `resolve_style_type()`: look up `LangItems` for intrinsic-backed properties |

## Pipeline for `padding: Pixel(value: 8)`

```
Source:  padding: Pixel(value: 8)
  │
  ▼ Parser:     property "padding", expression Pixel(value=8)
  ▼ HIR:        expected_type = Pixel (via LangItems["px"])
                expression type = Pixel
  ▼ Checker:    Pixel == Pixel → PASS
  ▼ Codegen:
      lower_expression → IR struct Pixel { value: 8 }
      extract .value  →  int(8)
      store in struct field  →  field[2] = int(8)
      sapply(PADDING, [comp, int(8)])
  ▼ Backend:     PADDING → "8px"
```

## Pipeline for `backgroundColor: Rgba(r: 255, g: 0, b: 0, a: 255)`

```
Source:  backgroundColor: Rgba(r: 255, g: 0, b: 0, a: 255)
  │
  ▼ Parser:     property "backgroundColor", expression Rgba(...)
  ▼ HIR:        expected_type = Color (via LangItems["color"])
                expression type = Color (= Rgba struct)
  ▼ Checker:    Color == Color → PASS
  ▼ Codegen:
      lower_expression → IR struct Rgba { r: 255, g: 0, b: 0, a: 255 }
      pack → int(0xFF0000FF)   //  (255 << 24) | (0 << 16) | (0 << 8) | 255
      store in struct field  →  field[0] = int(0xFF0000FF)
      sapply(BACKGROUND_COLOR, [comp, int(0xFF0000FF)])
  ▼ Backend:     BACKGROUND_COLOR → "rgba(255, 0, 0, 1)"
```

## Adding a new intrinsic-backed property (example)

```slynx
@intrinsic('px')
object Pixel { value: int }

stylesheet Padding(value: Pixel) {
    styles {
        padding: value
    }
}
```

Steps:

| Step | File | Change |
|---|---|---|
| 1 | `crates/hir/src/implementation/statements.rs` | Add `"padding" => self.get_lang_type_id("px").unwrap_or_else(|| self.int32_type())` |
| 2 | `crates/ir/src/model/styles.rs` | Add `Padding = 2`, update `from_name`, `ir_type` (returns `int`), `Display` |
| 3 | `crates/codegen/src/helper/styles.rs` | Already generic — extraction handles `Pixel` automatically via single-field rule |

## Implementation order

1. Add `get_lang_type_id` helper to `SlynxHir` (`crates/hir/src/lib.rs`)
2. Add `Padding`/`Margin` to `StyleProperty` (`crates/ir/src/model/styles.rs`)
3. Extend `resolve_style_type` to look up `LangItems` (`crates/hir/src/implementation/statements.rs`)
4. Add extraction logic to `create_style_constructor` (`crates/codegen/src/helper/styles.rs`)
5. Optionally add `lower_raw` for multi-field packing (`crates/ir/src/model/styles.rs`)
6. Update STYLES_TABLE.md with new property codes

## Intrinsics On Styles
Intrinsic types on the styles are handled via getting the fields. For example, if a style application expects [int32,int32, float32], then the object will follow so, and be such as `object O {f1: int32, f2: int32, f3: float32};`.

Its going to be made like this since it will make things easier to simply compile down. Thus f1 = argument(0), f2 = argument(1) and f3 = argument(3). So it's mainly positional
