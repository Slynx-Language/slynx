# Reference Expressions

## Overview

Reference expressions are constructed with the prefixed operator `&` (immutable reference) or `&mut` (mutable reference). They produce a reference to the underlying value without transferring ownership, allowing values to be passed by reference instead of being moved.

```slynx
&expr     // immutable reference
&mut expr // mutable reference
```

## Main Idea

References are cheap-to-copy values that point to an existing value without claiming ownership. They follow borrow checker rules inspired by Rust:

- multiple immutable references can coexist;
- only one mutable reference can exist at a time;
- a mutable reference cannot coexist with immutable references to the same value.

## Syntax

```slynx
func main(): void {
    let a = 5;
    let b = &a;       // immutable reference
    let c = &mut a;   // error: `a` is immutable
}

func main2(): void {
    let mut p = Person(name: "João");
    let pref = &mut p;    // mutable reference
    pref.name = "Ana";    // write through the reference
}
```

## Reference Types

| Expression | Resulting Type | Can Read | Can Write |
|-----------|-----------------|----------|---------------|
| `&expr` | `&T` | Yes | No |
| `&mut expr` | `&mut T` | Yes | Yes (if the value is mutable) |

## Rules

- `&expr` requires `expr` to be a place (variable, field access, index, deref).
- `&mut expr` additionally requires `expr` to be mutable (`let mut` or a field of a mutable value).
- Creating a mutable reference from an immutable variable is an ownership error.
- After a reference is created, the referenced value cannot be moved while the reference is alive.
- References can be returned from functions: `func f(): &int { return &a; }`.

## Examples

```slynx
object Person {
    name: str,
}

func main(): void {
    let mut a = 5;
    let mut p = Person(name: "John");

    let aref = &mut a;     // mutable reference to `a`
    *aref = 10;            // write via dereference

    let pref = &mut p;     // mutable reference to the object
    pref.name = "";        // write via field access on the reference
}
```

Multiple immutable references are allowed:

```slynx
func main(): int {
    let x = 42;
    let r1 = &x;
    let r2 = &x;
    let r3 = &x;
    r3
}
```

## Interaction

- The parser produces `ASTExpression::Reference { mutable, expr }` (`crates/parser/src/expr.rs`).
- The HIR represents references with `HirType::ImutableRef`/`HirType::MutableRef` for types and `ExpressionUse::Borrow`/`BorrowMut` in use analysis.
- In the IR, references are pointers (`IRType::Pointer`).
- Ownership analysis (`crates/hir/src/ownership/`) verifies exclusivity of `&mut` and coexistence of `&`.
- Dereferencing is described in [dereference.md](dereference.md).
- Reference types in signatures are described in [move-semantics.md](move-semantics.md).

## Summary

- `&expr` creates an immutable reference of type `&T`.
- `&mut expr` creates a mutable reference of type `&mut T`.
- Exclusivity rules: many `&T`, a single `&mut T`, never both.
- References do not transfer ownership; moving the underlying value while references are alive is an error.
