# Move Semantics

## Overview

Slynx uses an ownership and move semantics model inspired by Rust. Values are **moved** when transferred to another owner; after the move, the previous owner can no longer use the value. To avoid copying large values, the language offers reference types — cheap to copy and that do not transfer ownership.

The model does **not** rely on RAII for memory management: ownership and lifetime are tracked explicitly by the compiler. There are no automatic `drop`/destructors in the current model.

## Main Idea

The compiler tracks, for each value, who its owner is at each point in the program. When a value is moved, ownership is transferred and subsequent use by the previous owner is an error.

```slynx
let a: T = ...;
let b: T = a;   // move: `a` transfers ownership to `b`
// `a` can no longer be used
```

```
Before:
a ─────► T

After:
a       b ─────► T
                ownership
```

## Reference Types

To avoid moving/copying large values, the language provides references. References do not transfer ownership.

### `&T` — shared immutable reference

```slynx
let value: T = ...;
let reference: &T = &value;
```

- does not own the value;
- provides read-only access;
- can coexist with other `&T` to the same value;
- cannot mutate the value.

```
        ┌─────────┐
        │    T    │
        └─────────┘
          ↑  ↑  ↑
          │  │  │
         &T &T &T
```

### `&mut T` — exclusive mutable reference

```slynx
let mut value: T = ...;
let reference: &mut T = &mut value;
```

- does not own the value;
- provides read and write access;
- must be **unique** while it exists;
- does not coexist with any other `&T` or `&mut T` to the same value.

```
        ┌─────────┐
        │    T    │
        └─────────┘
             ↑
             │
           &mut T
```

### Fundamental rule

> Either there are multiple shared references `&T`, or there is exactly one mutable reference `&mut T` — never both at the same time on the same value.

### `&atomic T` — planned

A third type `&atomic T` is documented in the language design: an atomic/synchronized reference that allows concurrent read and write access and can coexist with other `&atomic T`. **Note**: `&atomic T` is **not yet implemented** in the parser, the type system, or the ownership analysis. Only `&T` and `&mut T` exist today.

## Ownership and Moves

### Move by assignment

```slynx
let a: T = valor;
let b: T = a;   // move: ownership transferido
```

After the move, `a` is no longer a valid owner and cannot be used as if it still were.

### Copy Types

Primitive types (`int`, `float`, `bool`, `str`) are **Copy**: they are implicitly duplicated, without a move.

### Move-only Types

Structs (objects), tuples, arrays, vectors, enums are **Move-only**: assigning or passing transfers ownership.

### Moves via function calls

Passing a moved value by value to a function moves the value:

```slynx
take(a);   // move: `a` is transferred to `take`'s parameter
```

### Borrow after move

```slynx
let b = a;    // move
let r = &a;   // error: `a` was already moved
```

## Ownership Analysis Rules

| Situation | Status |
|----------|--------|
| Use after move (`let b = a; let c = a;`) | **Error** (`UseAfterMove`) |
| Borrow after move (`let b = a; let r = &a;`) | **Error** |
| Conflicting borrow (mutable + immutable on the same value) | **Error** (`ConflictingBorrow`) |
| Move while borrowed | **Error** (`MoveWhileBorrowed`) |
| `&mut` on an immutable variable | **Error** (`MutablyBorrowImmutable`) |
| Multiple `&T` on the same value | Allowed |
| Single `&mut T` on a value | Allowed |

## Examples

Valid:

```slynx
object Person {
    name: str,
}

func main(): void {
    let mut a = 5;
    let mut p = Person(name: "John");

    let aref = &mut a;
    let pref = &mut p;
    *aref = 10;
    pref.name = "";
}
```

Invalid (use after move):

```slynx
func main(): void {
    let a = 5;
    let b = a;   // `a` is moved
    let c = a;   // erro: use after move
}
```

Invalid (borrow after move):

```slynx
func main(): void {
    let a = 5;
    let b = a;   // move
    let r = &a;  // error: `a` was already moved
}
```

Invalid (mutable borrow of immutable):

```slynx
func main(): void {
    let a = 5;
    let r = &mut a;  // error: `a` was not declared as mutable
}
```

## Places and Tracking

The analysis operates on **places** (`HirPlace`):

| Place | Example |
|-------|---------|
| `Variable` | `x` |
| `Temporary` | intermediate results |
| `Field` | `obj.field` |
| `Index` | `arr[i]` |
| `Deref` | `*ptr` |

The state of each place is tracked via `PlaceState` to detect moves, conflicting borrows, and use after move.

## Interaction

- Parser: `&expr`, `&mut expr`, `*expr` expressions (see [reference-expressions.md](reference-expressions.md), [dereference.md](dereference.md)).
- Types: `&T`, `&mut T` in signatures.
- Analysis: `crates/hir/src/ownership/` runs after HIR generation.
- IR: references become pointers; opcodes `Deref`/`DerefWrite`/`Ref`/`FieldRef`.

## Summary

| Type | Ownership | Read | Write | Multiple references |
|------|-----------|------|-------|------------------------|
| `T` | Owns | Yes | Yes, if mutable | N/A |
| `&T` | No | Yes | No | Yes |
| `&mut T` | No | Yes | Yes | No |
| `&atomic T` | No (planned) | Yes | Yes | Yes (planned) |

- Copy: `int`, `float`, `bool`, `str`.
- Move-only: objects, tuples, arrays, vectors, enums.
- `&atomic T` is a documented design, **not** implemented.
