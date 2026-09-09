# The `Self` Type

## Overview

Inside an object method, `Self` refers to the type of the object itself. It is used in the return type of constructors (`func new(): Self`) and in methods that receive or return instances of the same type.

## Main Idea

`Self` is an alias for the type of the object that contains the method. This avoids repeating the object name within methods, which is especially useful for constructors, instance methods, and when the object name is long.

## Syntax

```slynx
object Counter {
    count: int,

    func new(): Self {
        Self(count: 0)
    }

    func add(&mut self, other: Self): void {
        self.count = self.count + other.count;
    }
}
```

## Uses

- **Constructor return type**: `func new(): Self`.
- **Instance method return type**: `func clone(&self): Self`.
- **Parameter that is an instance of the same type**: `func add(&mut self, other: Self)`.
- **Construction within methods**: `Self(count: 0)` — construction call of the type itself.

## Rules

- `Self` is only valid inside the body of an object (methods).
- Outside an object, `Self` is not recognized as a type.
- `Self` resolves to the concrete type of the declared object.
- In reference contexts, reference-prefixed types also apply: `&Self`, `&mut Self`.

## Examples

```slynx
object Rgba {
    r: int, g: int, b: int, a: int,

    func new(r: int, g: int, b: int, a: int): Self {
        Self(r: r, g: g, b: b, a: a)
    }

    func white(): Self {
        Self.new(255, 255, 255, 255)
    }

    func negative(&self): Self {
        let white = Rgba.white();
        white.minus(*self)
    }
}
```

## Interaction

- The parser recognizes `self`/`Self` as a special identifier in typed parameters (`crates/parser/src/types.rs`).
- `self` (parameter) has type `Self`, `&self` has type `&Self`, and `&mut self` has type `&mut Self`.
- Methods are lowered to functions with an explicit receiver as the first parameter (see [struct-methods.md](struct-methods.md)).

## Summary

| Usage | Example |
|-----|---------|
| Type of the object itself | `func new(): Self` |
| Internal construction | `Self(r: 0, g: 0, b: 0, a: 0)` |
| Receivers | `self`, `&self`, `&mut self` |
| Same-type parameter | `func add(&mut self, other: Self)` |
