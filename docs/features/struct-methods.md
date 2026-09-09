# Struct Methods (Object Methods)

## Overview

Objects can declare methods — functions associated with the object's type — within their body. Methods can be instance methods (receive `self`) or type-associated (constructors/factories, without `self`).

## Main Idea

Methods allow grouping behavior with data. Syntactically, they are declared with `func` inside the object body:

```slynx
pub object Counter {
    count: int,

    func new(): Self { ... }              // type-associated method (no receiver)
    func increment(&mut self, amount: int): void { ... }  // instance method
    func getValue(&self): int { ... }     // instance method
}
```

Calls use dot syntax: `counter.increment(5)`, `counter.getValue()`, `Counter.new()`.

## Receiver Forms

| Receiver | Type | Can read | Can write |
|----------|------|----------|-----------|
| `self` | `Self` (by value) | Yes | Yes (own value) |
| `&self` | `&Self` | Yes | No |
| `&mut self` | `&mut Self` | Yes | Yes |

A method without a receiver is a **type-associated method** — typically used for constructors and factories.

## Type-associated methods (constructors/factories)

```slynx
pub object Counter {
    count: int,

    func new(): Self {
        Self(count: 0)
    }

    func staticFactory(x: int, s: str): Self {
        Self(count: x)
    }
}
```

Usage: `Counter.new()`.

## Instance methods

```slynx
pub object Counter {
    count: int,

    func increment(&mut self, amount: int): void {
        self.count = self.count + amount;
    }

    func add(&mut self, other: Self): void {
        self.count = self.count + other.count;
    }

    func getValue(&self): int {
        self.count
    }
}
```

Usage:

```slynx
func main(): int {
    let mut v = Counter.new();
    v.increment(5);
    v.add(Counter.new());
    v.getValue()
}
```

## Rules

- Methods are declared with `func` inside the object body.
- The receiver (if any) is the first parameter and is called `self` (`self`, `&self` or `&mut self`).
- Methods without a receiver are type-associated (called as `Type.method()`).
- `Self` refers to the object type inside methods (see [self-type.md](self-type.md)).
- Methods can be `pub` and support attributes.
- Methods can accept instances of their own type as parameters: `func add(&mut self, other: Self)`.

## Lowering

Methods are lowered to ordinary functions with the receiver as an explicit first parameter:

- `self` → `self: Self`
- `&self` → `self: &Self`
- `&mut self` → `self: &mut Self`

Conceptual lowering example:

```slynx
// obj.getValue()  →  getValue(&obj)
// v.add(other)    →  add(&mut v, other)
// Counter.new()   →  new()
```

The receiver retains the same ownership/mutability rules as the original signature.

## Interaction

- Parser: `parse_method` in `crates/parser/src/objects.rs`.
- The `self`/`&self`/`&mut self` receiver is recognized in `parse_typedname` (`crates/parser/src/types.rs`).
- Methods are stored in `ObjectMethod` (name, args, return, attributes).
- Objects can be declared as `extern` — in which case methods have no body (see [externs.md](externs.md)).

## Summary

| Receiver | Type | Common use |
|----------|------|------------|
| (none) | Type-associated method | Constructor/factory: `Counter.new()` |
| `self` | `Self` | Methods that consume the value |
| `&self` | `&Self` | Read-only methods |
| `&mut self` | `&mut Self` | Mutating methods |

Methods are lowered to functions with an explicit receiver; they do not require special runtime representation.
