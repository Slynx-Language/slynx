# Component Initialization Lowering

This document describes how a component declaration (with `Div`/`Text` children and optional `style:` clauses) is lowered into SlynxIR.

## Overview

Every component declaration with at least one specialized child (Div or Text) generates:

1. An **init function** (e.g. `PedrinhoInit`) — sets up the **first** specialized child's built-in properties (text content, nested children)
2. An **`@initcall`** to that init function — registered as the component's first UI instruction
3. For **every** specialized child (including the first) that has a `style:` clause, a **`@initcall`** to the style's apply function — references the style's constructor call result

## Data Flow

### `initialize_component` (entry point)

File: `crates/slynx_ir/src/ir/components.rs`

This is called from `SlynxIR::generate()` for every `HirDeclarationKind::ComponentDeclaration`.

#### Phase 1 – Top-level Values

For each child declared in the component body:

- A `Component` IR instruction is created to hold the child's type
- All specialized children (Div/Text) are collected into `specialized_children` for later init/style handling
- Children values are stored in the component's `values` list

#### Phase 2 – Init Function

If the first child is specialized:

1. Switch to the init function context (e.g. `PedrinhoInit`)
2. Add `p0` as a `FuncArg` typed as the specialized child's type (`@div` or `@text`)
3. Call `emit_specialized_component_init` which emits `SetField` instructions for:
   - **Text**: sets field 0 to the text string
   - **Div**: sets fields 0..N to the child component values
4. Emit `ret`
5. Restore the main function context

#### Phase 3 – UI Instructions

Still inside the `if let Some((_, first_spec)) = specialized_children.first()` block:

1. **Constructor calls**: For **every** specialized child that has a `style:` clause, emit a `FunctionCall` instruction to that style's **constructor** (e.g. `__init_Maria()`) — this produces the style struct value. These instructions are **unmapped** and stored in the global instruction pool, but NOT in `ui_instruction`. Each creates a `Value::new_instruction` pointing to the constructor result.

2. **Component initcall**: Emit `@initcall ComponentInit, #t0` — always present, with 1 operand (the first child's component value).

3. **Style initcalls**: For **every** specialized child with a `style:` clause, emit `@initcall ApplyStyle, #t_idx, <struct_value>` — 2 operands: the child component and its constructor result. The `#t_idx` references the correct child (not always `#t0`).

4. Set `ui_instruction` to a contiguous range spanning the component initcall + all style initcalls (the constructor calls are before them and are NOT included).

### `get_component_expression`

This function lowers a component expression into an IR value, emitting `SetField` instructions for its properties and children. It is called:

- **Inside `initialize_component`** (via `emit_specialized_component_init`) for top-level children
- **Inside function bodies** (via `get_value_for`) for inline component expressions

It returns `(Value, Option<StyleApplyData>)` — the component value and, if applicable, the style's init/apply function pointers. In function-body contexts the style data is discarded (the parent component handles it at a higher level).

## IR Instruction Types Involved

### `@initcall` (1 operand)

```
@initcall FuncName, Component;
```

Calls `FuncName(Component)` where `FuncName` is the component's init function that sets up children/text.

### `@initcall` (2 operands)

```
@initcall ApplyFunc, Component, ConstructorCall();
```

Calls `ApplyFunc(Component, StructValue)` where:
- `ApplyFunc` is the style's apply function (emits `@sapply` for each property)
- The second operand is a `Value` that points to a `FunctionCall` instruction (the constructor)

The formatter inlines the constructor call into the initcall display:

```slynxir
@initcall Maria, #t0, __init_Maria();
```

### `FunctionCall`

```
__init_Maria()
```

A regular function call instruction used to invoke the style constructor. It is stored in the global instruction pool but is NOT part of `ui_instruction` — it only exists as a referenced value inside the style's `@initcall`.

## Code Locations

| Concern | File | Function |
|---------|------|----------|
| Component init entry point | `crates/slynx_ir/src/ir/components.rs` | `initialize_component` |
| Specialized child property setup | `crates/slynx_ir/src/ir/components.rs` | `emit_specialized_component_init` |
| Component expression lowering | `crates/slynx_ir/src/ir/components.rs` | `get_component_expression` |
| Style resolution to IR pointers | `crates/slynx_ir/src/ir/components.rs` | `get_style_application` |
| Stylesheet lowering (struct + fns) | `crates/slynx_ir/src/ir/helper/styles.rs` | `lower_stylesheet` |
| `StyleApplyData` | `crates/slynx_ir/src/ir/components.rs` | struct definition |
| `AuxiliaryStyle` (hoisted data) | `crates/slynx_ir/src/ir/temp.rs` | struct definition |
| Hoisting pass | `crates/slynx_ir/src/ir/mod.rs` | `generate` |
| Formatter for `@initcall` | `crates/slynx_ir/src/ir/visualize/formatter.rs` | `format_instruction` |

## Example Walkthrough

### Source

```slynx
stylesheet Maria(){
    styles {
        default {
            backgroundColor: 0xff0000
        }
    }
}
component Pedrinho {
    Div {
        style: Maria(),
        Text {
            text: "Pedrinho leitazedo"
        }
    }
}
```

### Generated IR

```
struct %Maria{i32};

%Maria __init_Maria(){
$label_0:
  ret %Maria{16711680};
}

void Maria(anycomponent, %Maria){
$label_0:
  %t3 = getfield i32, p1, 0;
  %t4 = @sapply BACKGROUND_COLOR, p0, %t3;
  ret void;
}

void PedrinhoInit(@div){
$label_0:
  %t7 = @text{};
  propset %t7, 0, "Pedrinho leitazedo";
  propset p0, 0, %t7;
  ret void;
}

component %Pedrinho() {
  #t0: @div;
  @initcall PedrinhoInit, #t0;
  @initcall Maria, #t0, __init_Maria();
}
```

The constructor call `__init_Maria()` is in the global instruction pool. The formatter reads it via the second operand's `Value::Instruction` pointer and displays it inline inside the style initcall.

### Multiple specialized children with styles

```slynx
component Example {
    Div   { style: A(), Text { text: "first" } }
    Text  { style: B(), "second" }
    Div   { style: C(), Text { text: "third" } }
}
```

Generates:

```slynxir
component %Example() {
  #t0: @div;
  #t1: @text;
  #t2: @div;
  @initcall ExampleInit, #t0;
  @initcall A, #t0, __init_A();
  @initcall B, #t1, __init_B();
  @initcall C, #t2, __init_C();
}
```

Each specialized child that has a `style:` clause gets its own `@initcall` with the correct child reference (`#t0`, `#t1`, `#t2`).

## Limitations

- **Non-first specialized children have no init function.** Only the first specialized child gets an init function (e.g. `ComponentInit`) for setting up its built-in properties. If later specialized children need child/text property setup, they do not get it — only the style is applied.
- **Nested children's styles are not applied.** The style on a `Text` or `Div` that is a child of another specialized `Div` is ignored. For example, `Div { style: Outer(), Text { style: Inner(), text: "hi" } }` — `Outer()` is applied but `Inner()` is not.
- Styles on non-specialized components (custom components used with `style:`) are not supported.
