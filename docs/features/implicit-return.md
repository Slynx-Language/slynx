# Implicit Return

## Overview

In Slynx, the last expression of a block function body acts as the function's return value, without needing `return`. This is "implicit return".

## Main Idea

A function body in block form `{ ... }` produces a value when its last expression has the expected type. This allows functions to be written concisely without `return`.

```slynx
func add(a: int, b: int): int {
    let total = a + b;
    total            // implicit return
}
```

## Rules

- The last expression of a function block is the return value.
- If the function has a `void` return type, the block does not need a final expression with a value (it can simply have none).
- If the function is non-`void`, the block must end with an expression that produces a value (implicit return) or with `return expr`.
- The final expression does not need `;` — it is the value of the block.

## Examples

Implicit return of a calculation:

```slynx
func bigger(a: int, b: int): bool {
    a > b && a >= b
}
```

Implicit return of an object:

```slynx
object Person {
    name: str,
    age: int,
}

func main(): Person {
    Person(name: "João", age: 22)
}
```

Combining statements and implicit return:

```slynx
func main(): int {
    let value = 1;
    let mut total: int = 2;
    total = value + total;
    total
}
```

## Interaction

- The concise alternate return form is the expression body with `->`: `func f(): int -> expr;` (see [language-surface.md](../language-surface.md) — Functions).
- Implicit return is a block convention; explicit `return` is still supported (see [return-statements.md](return-statements.md)).
- The HIR/type-checker verifies that the last value of the block matches the declared return type when the function does not end with `return`.

## Summary

| Form | Example | Equivalent |
|-------|---------|-------------|
| Implicit return | `func f(): int { 42 }` | `func f(): int -> 42;` |
| Explicit return | `func f(): int { return 42; }` | — |
| Expression body | `func f(): int -> 42;` | — |
