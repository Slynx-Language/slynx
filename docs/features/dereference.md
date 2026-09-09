# Dereference

## Overview

The unary `*` operator dereferences a reference, producing access to the pointed-to value. It is the counterpart of the reference operation `&` (see [reference-expressions.md](reference-expressions.md)).

## Main Idea

Given a reference to a value (immutable or mutable), dereferencing allows reading — and, if the reference is mutable and the value is mutable, also writing — the underlying value.

## Syntax

```slynx
*expr
```

Dereferencing can also appear as an assignment target (left side of `=`):

```slynx
*ref = value;
```

## Rules

- `*expr` is only valid when `expr` has a reference type (`&T` or `&mut T`).
- Reading through `*ref` is possible with both immutable and mutable references.
- Writing through `*ref = value` requires the reference to be `&mut T` and the pointed-to value to be mutable.
- The type of `*expr` is the pointed-to type `T`.

## Examples

```slynx
func main(): void {
    let mut a = 5;
    let aref = &mut a;
    *aref = 10;   // writes to the pointed value
}

func main2(): int {
    let x = 42;
    let ref = &x;
    let y = *ref;  // reads the pointed value (42)
    y
}
```

## Interaction

- The parser converts `*expr` into `ASTExpression::Deref` (`crates/parser/src/expr.rs`).
- The HIR represents dereferencing with a deref `HirExpression`.
- In the IR, there are `Deref` (read) and `DerefWrite` (write) opcodes.
- Ownership analysis checks that the dereferenced place is mutable when writing.
- Field dereferencing: `obj.field = value` is parsed as a direct field assignment, while `*ref = value` is explicit dereferencing.

## Summary

| Operation | Syntax | Requirement |
|-----------|--------|-------------|
| Read via dereference | `let y = *ref;` | `ref: &T` or `&mut T` |
| Write via dereference | `*ref = value;` | `ref: &mut T` and mutable value |
