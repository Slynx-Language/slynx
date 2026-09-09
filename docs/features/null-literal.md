# Null Literal

## Overview

The literal `null` represents the absence of value in a context where a nullable type is expected. It is the only value assignable to a nullable type beyond the values of the base type. See [nullable-types.md](nullable-types.md).

## Main Idea

A nullable type (`T?`) accepts two kinds of values:

- values of type `T` — for example, `int?` accepts `5`, `-3`, `0`;
- the literal `null`, which represents the absence of value.

`null` is not a type of its own; it can only be used where a nullable type is expected.

## Syntax

```slynx
let a: int? = null;
let b: int? = 15;    // value of the base type
let c: ( []int )? = null;   // nullable vector
```

## Rules

- `null` is only valid in a position that expects a nullable type (`T?`).
- Assigning `null` to a variable of non-nullable type is a type error.
- Nullable collection types require parentheses: `( []int )?`, `( [4]int )?`.
- The exact type of `null` is inferred from the context (`HirType::Nullable(inner)`).

## Examples

```slynx
func main(): void {
    let a: int? = null;
    let b: int? = 15;
    let v: ( []int )? = null;
}
```

## Interaction

`null` is a lexer token (`TokenKind::Null`) and produces the expression `ASTExpression::Null`. In the HIR, the `null` literal is constructed with the nullable type from the inference context. Coercion of the base value to `T?` is also handled in the HIR.

## Summary

- `null` is used exclusively in nullable type contexts.
- Type inference for `null` depends on the context where it appears.
- The same literal works for any nullable type (`int?`, `str?`, `([]int)?`).
