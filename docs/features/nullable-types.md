# Nullable Types

## Overview

Nullable types (`T?`) can represent both a value of type `T` and the absence of value (`null`). They are a mechanism for modeling optional values without needing a "presence/absence" enum.

## Main Idea

Any existing type can be made nullable by adding the `?` suffix:

- `int` → `int?`
- `str` → `str?`
- `( []int )` → `( []int )?`
- `[4]int` → `( [4]int )?`

A value of type `T` can be assigned to a variable of type `T?` automatically (implicit promotion). The literal `null` is the only absence value.

## Syntax

```slynx
let a: int? = null;
let b: int? = 15;          // base value promoted to nullable
let v: ( []int )? = null; // nullable of vector
```

## Rules

- The `?` suffix applies to any type.
- Nullable collection types require parentheses: `([]int)?`, `([4]int)?`.
- The value `null` is only accepted in a nullable type context.
- A value of base type `T` is accepted where a `T?` is expected.
- The reverse direction (using `T?` where `T` is expected) is not allowed without nullability checking.

## Examples

```slynx
func main(): void {
    let a: int? = null;
    let b: int? = 15;
    let s: str? = "hello";
    let arr: ( []int )? = null;
}
```

Beyond literals, nullable types can appear in other positions:

- generic type arguments: `identity<int?>(42)` — see [generic-functions.md](generic-functions.md);
- struct fields: `object Box<T> { data: T? }`;
- function return types: `func identity<T>(x: T): T?` — see [generic-functions.md](generic-functions.md).

## Interaction

- In the parser, `Type::Nullable(inner)` represents the nullable type (`crates/parser/src/ast/types.rs`).
- In the HIR, `HirType::Nullable(inner)` carries the base type.
- Type inference of the `null` literal uses the context to determine the target nullable type.
- Nullable types can be used as generic arguments and in component props.

## Summary

| Aspect | Value |
|---------|-------|
| Syntax | `T?` |
| Absence literal | `null` |
| Implicit promotion | yes (`T` → `T?`) |
| Nullable collections | require parentheses: `([]int)?` |
