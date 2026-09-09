# Generics

## Overview

Generics allow defining functions, objects, components, enums, and stylesheets that operate on one or more parameterized types, specialized at compile-time by **monomorphization**.

## Main Idea

When a generic type `T` is instantiated with arguments like `int`, `str`, or `Option<int>`, the compiler generates a **specialized copy** of the code with `T` replaced by the concrete type. This produces fully optimized code, at the cost of code duplication per specialization.

Conceptual example:

```slynx
struct Vector<K> { ... }
```

If the code uses `Vector<int>`, the compiler "copy-pastes" the implementation of `Vector` replacing `K` with `int`, generating `Vector<int>`. The same applies to `Vector<str>` etc.

## Where generics can be declared

| Construct | Syntax | Document |
|-----------|--------|----------|
| Function | `func name<T>(x: T): T` | [generic-functions.md](generic-functions.md) |
| Object | `object Name<T> { field: T }` | [generic-structs.md](generic-structs.md) |
| Component | `component Name<T> { pub prop x: T }` | [generic-components.md](generic-components.md) |
| Enum | `enum Name<T> { Some(T) }` | [generic-enums.md](generic-enums.md) |
| Stylesheet | `stylesheet Name<T>(x: T)` | [generic-stylesheets.md](generic-stylesheets.md) |

## Rules

- Type parameters are declared in the `<T, U, ...>` list after the name.
- Inside the body, the parameters can be used as field, argument, return, and prop types.
- At the call site, type arguments must be provided **explicitly**.
- The number of arguments must equal the number of parameters.
- Each distinct combination of type arguments generates a specialization.
- Identical specializations are deduplicated (one copy per combination).
- There is no automatic type inference at the call site.
- There are no bounds, trait bounds, const generics, or default values for parameters at this time — these are planned future extensions.

## Current limitations

- **Type inference**: `identity(65)` without an explicit `<int>` does not work.
- **Argument validation**: type validation of arguments at the call site is incomplete in generic cases (`xpass` tests).
- **Generic object methods**: specialized structs are created with an empty method table (generic object methods may not be fully connected).
- **Generic type aliases**: generic `alias` are not specialized by the monomorphizer.
- **Generic statics**: not supported by the monomorphizer.

## Monomorphization

The process is implemented in the `crates/monomorphizer/` crate:

1. Find all instantiations of generic types in the program.
2. For each combination of type arguments, generate a specialization.
3. Name the specialization deterministically: `<template>_<arg0-name>_<hash>`.
4. Deduplicate identical specializations.
5. Neutralize the templates (empty the body, signature `()->void`) so that codegen does not process them.
6. Detect instantiation cycles (infinite recursion) and report errors.

## Deduplication

Calls to `identity<int>` in different locations of the same program generate **a single** specialization thanks to a cache (`DashMap`) indexed by `(template, type_args)`.

## Dead code

After specialization, the original templates are marked as dead code; codegen ignores them.

## Examples

```slynx
func identity<T>(x: T): T {
    x
}

object Option<T> {
    value: T,
}

component Box<T> {
    pub prop item: T,
}

enum Thing<T> {
    Some(T),
    None,
}

func main(): int {
    let a = identity<int>(65);
    let b = Option<int>(value: a);
    if b.value matches 65 {
        1
    } else {
        0
    }
}
```

## Interaction

- In the parser, `Type::Generic(index)` represents usage of a parameter inside a generic body.
- Generic type name resolution uses `is_generic_application()` to distinguish `Name<...>` from comparisons `a < b`.
- The HIR carries `HirType::GenericParam { name, variance }`.
- Codegen should never see `HirType::GenericParam` — the monomorphizer neutralizes it beforehand.

## Summary

- Generics are implemented via monomorphization (copy per specialization).
- Applicable to functions, objects, components, enums, and stylesheets.
- Explicit type arguments are required.
- Deduplication of identical specializations.
- No inference, bounds, const generics, or default parameters.
