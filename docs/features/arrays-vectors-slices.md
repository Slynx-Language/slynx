# Arrays, Vectors and Slices

## Overview

Slynx has two basic collection types — **arrays** (fixed size) and **vectors** (dynamic size) — plus **slice**/indexing operations that produce views over them.

## Main Idea

| Collection | Syntax | Size |
|------------|--------|------|
| Array | `[N]T` — e.g.: `[64]int` | Fixed, known at compile-time |
| Vector | `[]T` — e.g.: `[]int` | Dynamic (unknown at compile-time) |
| Slice | view over array/vector | via indexing expressions |

## Types

```slynx
let a: [4]int;      // array of 4 ints
let b: [64]bool;    // array of 64 bools
let c: []int;       // vector of ints (dynamic size)
let d: []Component; // vector of components
```

## Creation

### Array literal

```slynx
let a: [4]int = [1, 2, 3, 4];
```

The type of an array literal is always `[N]T` — the expected type does not change this. To create a `[4]int` from values, use `let a: [4]int = [1,2,3,4]`.

### Vector literal

```slynx
let b: []int = {};
let f: []int = {1, 2, 3};
```

The type of a vector literal is always `[]T`. Vectors use curly braces `{}` for their literals.

### Fill

A literal prefixed by `[N]` followed by a value repeats the value N times:

```slynx
let zeros: [44]int = [44]0;   // 44 zeros
```

## Indexing

```slynx
let first = arr[0];       // element access
let elem = &arr[1];       // reference to an element
```

## Slices

Slices are contiguous views over an array/vector, created by indexing expressions with ranges:

```slynx
expr[:]      // full slice
expr[N:]     // from index N to the end
expr[:M]     // from the start up to index M (exclusive)
expr[N:M]    // from index N up to index M (exclusive)
```

```slynx
let all = arr[:];       // all elements
let tail = arr[2:];     // from index 2 to the end
let head = arr[:2];     // from the start up to index 2 (exclusive)
let mid = arr[1:3];     // indices 1 and 2
```

### Slice rules

- Slices are operations that take a view of an array/vector; they can be understood as references.
- Slices are **temporary local values** — they should not be saved in structs.
- They can be used in functions and temporary parameters.
- A slice can be sliced again.

See [slicing.md](slicing.md) for the full range syntax documentation.

## Examples

```slynx
func main(): int {
    let a: [4]int = [1, 2, 3, 4];
    let b: []int = {};
    let f: []int = {1, 2, 3};

    let idx = a[0];
    let idx_ref = &a[1];

    let all = a[:];
    let tail = a[2:];
    let head = a[:2];
    let mid = a[1:3];

    a[0]
}
```

## Interaction

- Parser: arrays with `[`, vectors with `{`, and `IndexExpression` with `RangeType` (`crates/parser/src/expr.rs`).
- Types: `Array(inner, len)` and `Vector(inner)`.
- Nullable types wrapping collections require parentheses: `([]int)?`.
- In the IR, arrays/vectors have dedicated opcodes: `Array`, `Vector`, `ArrayGet`, `ArrayLen`, `VectorPush`.

## Summary

| Collection | Type | Literal | Size |
|------------|------|---------|------|
| Array | `[N]T` | `[1, 2, 3]` | Fixed |
| Vector | `[]T` | `{1, 2, 3}` | Dynamic |
| Slice | view | `arr[:]`, `arr[1:3]` | via ranges |

- Arrays: brackets; vectors: curly braces.
- Fill: `[44]0` repeats the value.
- Slices are temporary and cannot be saved in structs.
