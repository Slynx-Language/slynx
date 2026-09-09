# Generic Functions

## Overview

Functions can declare type parameters, allowing them to operate on values of any type that satisfies the constraints in the body. The parameterization is resolved at compile time via monomorphization: each concrete instance generates specialized code.

## Main Idea

A generic function declares type parameters between `<` and `>` after the name:

```slynx
func identity<T>(x: T): T { x }
```

At the call site, type arguments must be provided explicitly:

```slynx
identity<int>(65)
identity<bool>(true)
identity<str>("hello")
identity<f64>(1.5)
```

Each call with distinct type arguments produces a separate specialization. Calls with the same type arguments are deduplicated: two calls to `identity<int>` produce a single specialization.

## Syntax

```slynx
func name<T, U, ...>(args): ReturnType { body }
func name<T>(args): ReturnType -> expr;
```

## Rules

- Type parameters are introduced with the list `<T, U, ...>`.
- Inside the body, `T` can be used as the type of arguments, return values, and variables.
- Explicit type arguments are mandatory at the call site (automatic inference is not supported — calling `identity(65)` without a type causes a failure).
- The number of type arguments must equal the number of declared parameters (`wrong_arity`).
- No type validation is applied to arguments at the call site (the argument's type is assumed even if it differs from the parameter — test cases marked as `xpass`).

## Examples

```slynx
func identity<T>(x: T): T {
    x
}

func second<T, U>(first: T, second: U): U {
    second
}

func len<T>(items: [4]T): int {
    4
}

func lenVec<T>(items: []T): int {
    items.length
}

func main(): int {
    let a = identity<int>(65);
    let b = identity<bool>(true);
    let c = identity<str>("hello");
    let d = identity<f64>(1.5);
    let e = second<int, str>(1, "ok");
    second<int, int>(a, e)
}
```

Nested and chained calls:

```slynx
func wrap<T>(x: T): T { x }

func main(): int {
    // wrap<int> calls identity<int>; both specializations are deduplicated
    wrap<int>(identity<int>(42))
}
```

## Current Limitations

- Argument type inference is not supported (you must use `<T>` explicitly).
- There are no bounds/constraints on type parameters (no trait bounds).
- There are no default values for type parameters.
- Argument type validation at the call site is not performed for generic cases.

## Interaction

- Monomorphization is performed in the `crates/monomorphizer/` crate.
- The name of each specialization is mangled for deduplication: `<template>_<arg0-name>_<hash>`.
- Identical specializations are deduplicated into a single instance.
- Instantiation cycles (infinite type recursion) are detected and reported.
- After specialization, the generic template is "neutralized" (body emptied) so that codegen does not process it.
- Generic types can be nested (`Box<Option<int>>`).

## Summary

- Generic functions use `func name<T>(...)`.
- Type arguments are required at the call site.
- Monomorphization generates specialized code per type combination.
- Deduplication prevents redundant code generation.
