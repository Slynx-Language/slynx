# Attributes

## Overview

Attributes are compile-time metadata applied to declarations, written with the syntax `@name(...)` before the declaration. They are inspired by TypeScript decorators, but the metadata is fully `compile-time`.

## Main Idea

An attribute is attached to a declaration and can:

- register capabilities of the function/declaration (`@capabilities`);
- register a declaration as a **builtin**/lang item of the compiler (`@builtin`);
- be stored as metadata for future use.

## Syntax

```slynx
@name(arg0, arg1, arg2)
```

Attributes can be stacked:

```slynx
@capabilities("fs", "io")
@deprecated
func f() { ... }
```

The arguments are string literals.

## Existing attributes

### `@builtin("name")`

Registers the declaration as a lang item in the compiler:

```slynx
@builtin("color")
pub object Color {
    inner: int,
    ...
}
```

This is the mechanism used by the standard library for intrinsic types. The standard library reference uses `@intrinsic("color")`; in the HIR, the recognized attribute is `@builtin("name")`, which registers the declaration in `LangItems`.

### `@intrinsic("name")`

Form used in standard library files (e.g.: `lib/std/color.slx`):

```slynx
@intrinsic("color")
pub object Color { ... }
```

### `@capabilities(...)`

Declares a set of capabilities (effects) that the function/declaration requires:

```slynx
@capabilities(fs(write))
func writeConfig() { ... }
```

The idea is that any function that calls `writeConfig` also needs to declare `fs(write)`. The capabilities system is **designed**: it is stored, but permission checking is not complete yet.

## Where attributes can be used

| Construct | Support |
|-----------|---------|
| Functions | Yes |
| Objects | Yes |
| Components | Yes |
| Enums | Yes |
| Enum variants | Yes |
| Stylesheets | Yes |
| Statics | Yes |

## Rules

- Syntax: `@name(arg0, arg1, ...)` with string arguments.
- Attributes appear before the declaration.
- Multiple attributes can be stacked.
- Unknown attributes are stored but not processed.
- In the HIR, `@builtin("name")` registers the declaration in `LangItems`; `@capabilities(...)` is stored for future use; `@unknown(...)` is accepted.

## Examples

```slynx
@builtin("color")
pub object Color {
    inner: int,

    func new(r: int, g: int, b: int, a: int): Self {
        Self(inner: (r << 24) | (g << 16) | (b << 8) | a)
    }
}
```

## Interaction

- Parser: `parse_attributes` in `crates/parser/src/declarations.rs`.
- HIR: processing in `crates/hir/src/builders/attributes/mod.rs`.
- `@builtin` feeds `LangItems` (`crates/hir/src/context/lang_items.rs`), used in intrinsic type resolution.
- The standard library (`lib/std/`) uses `@intrinsic` to register the `Color` object.

## Summary

| Attribute | Function | Status |
|-----------|----------|--------|
| `@builtin("name")` | Registers lang item/builtin | Implemented |
| `@intrinsic("name")` | Registers intrinsic (std library) | Implemented |
| `@capabilities(...)` | Declares effects/capabilities | Designed, stored only |
| `@unknown(...)` | Any other attribute | Stored, no processing |
