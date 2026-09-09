# Generic Structs (Objects)

## Overview

Objects can declare type parameters, enabling the creation of polymorphic data structures like `Option<T>`, `Pair<T, U>`, and generic wrappers. Specialization follows the same monomorphization mechanism used for functions.

## Main Idea

A generic object declares its type parameters after the name:

```slynx
object Option<T> {
    value: T,
}
```

At the usage site, the type is instantiated with concrete type arguments:

```slynx
let opt = Option<int>(value: 42);
```

Each instantiation with distinct type arguments produces a specialized struct type.

## Syntax

```slynx
object Name<T, U, ...> {
    field1: T,
    field2: U,
}

Name<int, str>(field1: ..., field2: ...)
```

## Rules

- Type parameters may appear in field types.
- Instantiation uses the object construction syntax with named arguments.
- Field access works normally on generic instances: `some.value`, `p.second`.
- The number of type arguments must match the parameter list.
- Generic objects can be nested: `object Outer<T> { inner: Inner<T> }`.

## Examples

```slynx
object Option<T> {
    value: T,
}

object Pair<T, U> {
    first: T,
    second: U,
}

func main(): int {
    let some = Option<int>(value: 42);
    let pair = Pair<int, str>(first: 1, second: "ok");
    let nested = Option<Pair<int, str>>(value: pair);
    some.value
}
```

Generic objects can also appear as type arguments of components:

```slynx
component Box<T> {
    pub prop item: T,
}

func main(): Component {
    Box<Option<int>> {
        item: Option<int>(value: 42)
    }
}
```

## Current Limitations

- Field type validation during instantiation is not performed in all cases (`xpass` for certain type errors).
- Methods of specialized generic objects are created with an empty method table (the current implementation does not populate methods per instance).
- Non-generic methods on generic objects may not be fully connected.

## Interaction

- Monomorphization specializes the struct for each combination of type arguments.
- Identical specializations are deduplicated.
- The instance name is mangled: `<template>_<arg0-name>_<hash>`.
- Generic types can reference other generic types (`Box<Option<int>>`).

## Summary

- Generic objects use `object Name<T> { field: T }`.
- Instantiation via construction with type: `Name<int>(field: ...)`.
- Specialization via monomorphization with deduplication.
