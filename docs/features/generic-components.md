# Generic Components

## Overview

UI components can be parameterized with types, allowing the creation of reusable components that accept any type in their properties.

## Main Idea

A generic component declares type parameters after the name and uses them in the types of its properties:

```slynx
component Box<T> {
    pub prop item: T,
}
```

Instantiation provides concrete type arguments:

```slynx
Box<int> { item: 42 }
```

Each combination of type arguments generates a specialized version of the component.

## Syntax

```slynx
component Name<T, U, ...> {
    pub prop a: T,
    pub prop b: U,
}

Name<int, str> {
    a: ..., 
    b: ...
}
```

## Rules

- Type parameters can appear in prop types.
- The number of type arguments must equal the number of parameters.
- Type validation of props at instantiation may be incomplete (`xpass` cases for prop type mismatches).

## Examples

```slynx
component Box<T> {
    pub prop item: T,
}

component Pair<T, U> {
    pub prop first: T,
    pub prop second: U,
}

func main(): Component {
    Box<int> {
        item: 42
    }
}
```

Nullable and collection types as type arguments:

```slynx
component Box<T> {
    pub prop item: T,
}

func main(): Component {
    Box<int?> {          // nullable type argument
        item: 42
    }
}
```

```slynx
component Box<T> {
    pub prop item: T,
}

func main(): Component {
    Box<[]int> {                 // vector type argument
        item: {1, 2, 3}
    }
}
```

```slynx
component Box<T> {
    pub prop item: T,
}

func main(): Component {
    Box<[4]int> {                // array type argument
        item: [1, 2, 3, 4]
    }
}
```

## Interaction

- Monomorphization specializes the component per type argument combination.
- Generic components can accept generic structs as arguments: `Box<Option<int>>`.
- The `crates/monomorphizer/` crate handles component specialization.

## Summary

- Generic components use `component Name<T> { pub prop x: T }`.
- Instantiation: `Name<int> { ... }`.
- Type arguments can be primitive types, nullable, collections, or structs.
