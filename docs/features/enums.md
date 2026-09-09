# Enums

## Overview

Enums define a type that can have one of several possible values. Each value is called a **variant**. Variants can carry associated values of any type — including generic types.

## Main Idea

An enum is a discriminated union type: it stores which variant the value represents (the **tag**) and, if the variant carries a payload, the associated data.

```slynx
enum Option<T> {
    Some(T),
    None,
}
```

Creation uses the syntax `EnumName.VariantName`:

```slynx
let opt: Option<int> = Option.Some(42);
```

## Declaration

```slynx
enum Name {
    Variant1,                 // raw variant (no payload)
    Variant2 = expr,          // raw variant with a value
    Variant3(Type1, Type2),   // associated variant (positional payload)
    Variant4 { field: Type }, // struct variant (named payload)
}
```

### Four variant types

| Type | Syntax | Example |
|------|--------|---------|
| **Raw** | `Name` | `None`, `Idle`, `Done` |
| **RawValued** | `Name = expr` | `Zero = 0`, `Busy = 3` |
| **Associated** | `Name(T1, T2)` | `Some(4)`, `C(1, "s", 2.5)` |
| **Struct** | `Name { field: T }` | `Struct { name: str }` |

### Representation

Enums can declare a representation type after `:` — used for raw/raw-valued variants:

```slynx
enum Numbers: int {
    One,
    Two,
    Three,
}
```

Currently, `repr: int` is supported. `repr: str` is rejected by the compiler.

### Empty enums

```slynx
enum Empty {}
```

### Attributes on variants

Variants can carry attributes:

```slynx
enum Status {
    @deprecated
    Idle,
    Busy,
}
```

## Creation

```slynx
enum Status {
    Idle,
    Busy = 3,
    Done,
}

let a = Status.Idle;
let b = Status.Busy;
```

With payload:

```slynx
enum NetworkError {
    Timeout,
    NotFound,
    BadRequest(str, int),     // payload posicional
}

let err = NetworkError.BadRequest("bad method", 405);
```

With struct variant:

```slynx
enum Shape {
    Circle { radius: float },
    Rect { w: float, h: float },
}

let circle = Shape.Circle(radius: 1.5);
```

## Generic Enums

Enums can be generic:

```slynx
enum Thing<T> {
    Some(T),
    None,
}

enum Pair<A, B> {
    Both(A, B),
    OnlyA(A),
}
```

Each specialization with different type arguments generates a distinct type:

```slynx
let a = Thing.Some(12);      // Thing<int>
let b = Thing.Some("hi");    // Thing<str>
```

See [generic-enums.md](generic-enums.md) for details.

## Matches Expression

The `matches` expression checks whether an enum value corresponds to a variant and optionally to its inner values:

```slynx
if opt matches Some(44) {
    // opt is Some and its content is 44
}
```

### Forms

| Form | Syntax | Example |
|------|--------|---------|
| Variant without payload | `x matches Name` | `s matches Done` |
| Variant with payload | `x matches Name(expr)` | `opt matches Some(44)` |
| Struct variant | `x matches Name { field: expr }` | `x matches Foo { x: 1 }` |

### Rules

- `matches` returns `bool`.
- For variants with payload, the inner value is compared against the pattern value.
- The wildcard `_` does NOT work — it is treated as an identifier and equality is attempted.
- `num matches Some(mynum)` means "the value is `Some` and its content is equal to `mynum`".
- `matches` binds tighter than `&&`/`||` and looser than comparisons.

See [matches-expression.md](matches-expression.md) for full documentation.

## Examples

```slynx
enum Option<T> {
    Some(T),
    None,
}

func is_forty_two(opt: Option<int>): bool {
    opt matches Option.Some(42)
}

func main(): int {
    let opt = Option.Some(42);
    if is_forty_two(opt) {
        1
    } else {
        0
    }
}
```

Returning an enum from a function:

```slynx
enum Result {
    Ok(int),
    Err(str),
}

func div(a: int, b: int): Result {
    if b == 0 {
        Result.Err("division by zero")
    } else {
        Result.Ok(a / b)
    }
}
```

## Interaction

- The parser produces `EnumDeclaration` with `EnumVariant` (4 kinds) (`crates/parser/src/enums.rs`).
- In the HIR, the enum is represented with variants and discriminants.
- In the IR, enums become structs with `tag` + union of the variant payloads.
- The `matches` check is lowered via `Opcode::MatchesTag`.
- Monomorphization specializes generic enums.

## Known limitations

- Payload type mismatch validation in variants may not be rejected in all cases (`xpass` tests).
- Invalid patterns in `matches` and use of `matches` on non-enums are rejected in some cases but may slip through in others (`xpass`/`xfail` tests).

## Summary

| Variant | Syntax | Payload |
|---------|--------|---------|
| Raw | `VariantName` | none |
| RawValued | `VariantName = expr` | value (int) |
| Associated | `VariantName(T, U)` | positional payload |
| Struct | `VariantName { f: T }` | named payload |

- Creation: `EnumName.VariantName(args)`.
- Checking: `value matches VariantName(args)`.
- Enums can be generic, with `repr: int` and with per-variant attributes.
