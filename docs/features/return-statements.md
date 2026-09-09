# Return Statements

## Overview

The `return` statement terminates the execution of a function early, optionally producing a return value. It is the explicit exit mechanism of a function, complementing the implicit return (see [implicit-return.md](implicit-return.md)).

## Main Idea

A function can produce its return value in two ways:

1. **Explicit return** — using `return` to terminate execution at any point in the body.
2. **Implicit return** — the last expression of the block is used as the return value.

`return` can optionally carry a value. If no value is provided, the function must have a return type of `void`.

## Syntax

```slynx
return;         // return with no value (`void` functions)
return expr;    // return with a value
```

## Rules

- `return` without a value is only valid in functions whose return type is `void`.
- `return expr` is only valid in functions with a return type other than `void`.
- The type of the expression in `return expr` must be assignable to the declared return type of the function.
- A non-`void` function must end in an expression that produces a value (implicit return) or in `return expr`. `void` functions cannot end with values.

## Examples

```slynx
func mod(a: int, b: int): int {
    if a < b {
        return a;
    }
    return a % b;
}

func log(message: str): void {
    if message == "" {
        return;  // early exit without a value
    }
    // ... continua o corpo
}
```

## Interaction

`return` is parsed at the statement level (`crates/parser/src/statement.rs`) and produces the statement `ASTStatement::Return`. In the HIR, it becomes a termination instruction that ends the control flow of the current block.

## Summary

| Form | Usage | Target Function |
|-------|-----|-------------|
| `return;` | Exit without value | `void` |
| `return expr;` | Exit with value | Any non-`void` type |
