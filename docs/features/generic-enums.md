# Generic Enums

## Overview

Enums can declare type parameters, enabling types like `Option<T>` and `Result<T, E>` that carry values of varying types.

## Main Idea

A generic enum declares its type parameters after the name and uses them in variant payloads:

```slynx
enum Thing<T> {
    Some(T),
    None,
}
```

Construction and `matches` checks use instantiated types:

```slynx
let x = Thing.Some(42);
if x matches Thing.Some(42) { ... }
```

## Syntax

```slynx
enum Name<T, U, ...> {
    Variant(T),
    Variant2(T, U),
    Variant3,
}
```

## Rules

- Type parameters may appear in variant payloads.
- The same generic instance can be used with different specializations in the same file: `Thing.Some(12)` and `Thing.Some("hi")` produce distinct specializations.
- Generic enums can be used in function signatures: `func identity(x: Boxed<int>): Boxed<int>`.
- The number of type arguments must match at the usage sites.

## Examples

```slynx
enum Thing<T> {
    Some(T),
    None,
}

enum Pair<A, B> {
    Both(A, B),
    OnlyA(A),
}

enum Mixed<T, U> {
    First(T),
    Second(U),
    Both(T, U),
}

func main(): int {
    let a = Thing.Some(12);      // specialization Thing<int>
    let b = Thing.Some("hi");    // specialization Thing<str>
    let both = Pair.Both(1, "ok");
    match_check(a)
}

func match_check(x: Thing<int>): int {
    if x matches Thing.Some(12) {
        1
    } else {
        0
    }
}
```

Multiple specializations of the same generic enum:

```slynx
enum Thing<T> {
    Some(T),
    None,
}

func main(): int {
    let a = Thing.Some(12);
    let b = Thing.Some("hi");   // second specialization
    let c = Thing.None;         // variant without payload
    0
}
```

## Interaction

- Monomorphization specializes the enum by combining type arguments.
- The `matches` check is performed on the specialized instance.
- As with other generic types, identical specializations are deduplicated.

## Summary

- Generic enums use `enum Name<T> { Variant(T), ... }`.
- Distinct specializations coexist in the same file.
- Payloads may contain parametric types, concrete types, or other types defined in the program.
