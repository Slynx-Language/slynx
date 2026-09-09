# Matches Expression

## Overview

The `matches` expression checks whether an enum value corresponds to a specific variant and, optionally, whether its payload matches the provided values. It is the current entry point for pattern matching in the language.

```slynx
expr matches Pattern
```

## Main Idea

`matches` is a binary operator that returns `bool`. The left-hand side is an enum value; the right-hand side is a pattern that describes a variant and, optionally, its inner values.

The operator binds more tightly than `&&`/`||` and more loosely than comparisons. Thus, `a matches Foo && b` groups as `(a matches Foo) && b`.

## Pattern Forms

| Form | Syntax | Example |
|-------|---------|---------|
| Variant without payload | `VariantName` | `x matches None` |
| Variant with payload | `VariantName(expr)` | `x matches Some(4)` |
| Variant struct | `VariantName { field: expr }` | `x matches Foo { id: 1 }` |

## Behavior

- For variants without a payload, `matches` only checks whether the value is of that variant.
- For variants with a payload, the inner value of the variant is compared against the value provided in the pattern.
- The `_` (wildcard) pattern is NOT supported as a wildcard — it is treated as an identifier and an equality comparison is attempted.
- A pattern like `num matches Some(mynum)` means "the value is `Some` and its contents are equal to `mynum`".

## Precedence

`matches` sits in the precedence chain between comparisons and logical operators:

```text
comparison (== , <, >, <=, >=)
    ↓
matches
    ↓
&&, ||
```

Thus, `a == b matches C && d` groups as `((a == b) matches C) && d`.

## Examples

```slynx
enum Option<T> {
    Some(T),
    None,
}

func main(): int {
    let opt: Option<int> = Option.Some(42);

    if opt matches Some(42) {
        1
    } else {
        0
    }
}
```

With multiple states:

```slynx
enum Status {
    Idle,
    Busy,
    Done,
}

func is_done(s: Status): bool -> s matches Done;
```

## Current Limitations

- There is no functional `_` wildcard.
- There is no variable binding in patterns (patterns do not capture named values).
- There are no arbitrarily nested patterns; only the variant form + values.
- There is no validation that the pattern is a variant of the correct enum — this gap is known in the tests (some cases marked as `xpass`/`xfail`).

## Interaction

- The parser converts `lhs matches pattern` into `ASTExpression::Matches` (`crates/parser/src/expr.rs`).
- In the IR, codegen routes by variant using `Opcode::MatchesTag`.
- The expression works inside `if` conditions and other boolean expressions.

## Summary

- `matches` checks an enum variant and, optionally, inner values.
- Returns `bool`.
- `_` does not yet work as a wildcard.
- Precedence: between comparisons and logical operators.
