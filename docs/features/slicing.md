# Slicing

## Overview

Slicing is an indexing operation that produces a view (slice) of an array or vector. Syntactically, slices are indexing expressions with ranges, as follows:

```slynx
expr[:]      // full slice
expr[N:]     // from index N to the end
expr[:M]     // from the start up to index M (exclusive)
expr[N:M]    // from index N up to index M (exclusive)
```

## Main Idea

Just like simple indexing `a[i]`, slices are postfix expressions. The difference is that the index is replaced by a range, producing a contiguous view of the original elements instead of a single element.

## Syntax

The parser recognizes the following range formats:

| Syntax | Meaning | AST |
|---------|-------------|-----|
| `a[i]` | Element at index `i` | `NoRange` |
| `a[:]` | All elements | `All` |
| `a[i:]` | From index `i` to the end | `From` |
| `a[:i]` | From the start to index `i` (exclusive) | `To` |
| `a[i:j]` | From index `i` to index `j` (exclusive) | `Normal` |

## Rules

- Slicing applies to array and vector expressions.
- The resulting type of a slice is a slice/view type — in the array/vector models, slices are typed as a reference to the array/vector.
- Mutability rules follow reference rules: a slice can be immutable or mutable depending on the context.
- Slices should not be saved in structs (they are short-lived temporary values).

## Examples

```slynx
func main(): void {
    let arr: [5]int = [1, 2, 3, 4, 5];

    let all = arr[:];       // [1, 2, 3, 4, 5]
    let tail = arr[2:];     // [3, 4, 5]
    let head = arr[:2];     // [1, 2]
    let mid = arr[1:3];     // [2, 3]
    let single = arr[0];    // 1 (simple indexing)
}
```

## Interaction

- The parser converts the range syntax into `RangeType` (`crates/parser/src/expr.rs`).
- The resulting expression is an `IndexExpression` in the AST.
- In the HIR and IR, array/vector slices generate reference access to the range.

## Summary

| Syntax | Meaning |
|---------|-------------|
| `[:]` | All elements |
| `[N:]` | From index `N` to the end |
| `[:M]` | From the start to index `M` (exclusive) |
| `[N:M]` | From index `N` to index `M` (exclusive) |
| `[N]` | Simple indexing (single element) |
