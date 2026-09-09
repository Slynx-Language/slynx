# Tuples

## Overview

Tuples are anonymous sequences of values of potentially different types. Unlike objects/structs, they have no named fields and require no prior declaration — the type is the arity of the list itself.

## Main Idea

A tuple is defined by a list of types in parentheses:

```slynx
(str, int)
```

The corresponding value is created with the same parenthesized syntax:

```slynx
("John", 30)
```

Field access is positional, using `.N` where `N` is the value's index:

```slynx
let name = person.0;
let age = person.1;
```

## Syntax

### Type

```slynx
(T1, T2, T3, ...)
```

The type of the empty tuple is `()` (unit).

Tuples can be nested:

```slynx
((str, int), float)
```

### Literal

```slynx
(v1, v2, v3, ...)
```

### Access

```slynx
tuple.0
tuple.1
```

Chained access works:

```slynx
pair.0.age
```

Since `pair.0` is an object with a field `age`.

## Rules

- The access index must be an integer literal.
- Accessing an index beyond the tuple size is a type error (`pair.2` on a 2-element tuple).
- Accessing `tuple.0` on a value that is not a tuple is a type error.
- Tuples can contain objects, other tuples, and primitives.

## Examples

```slynx
func new_person(name: str, age: int) -> (str, int) {
    (name, age)
}

func main(): int {
    let person = new_person("John", 30);
    let name = person.0;   // "John"
    let age = person.1;    // 30
    age
}
```

Objects inside tuples:

```slynx
object Person {
    age: int,
}

func main(): int {
    let pair = (Person(age: 22), "ok");
    pair.0.age   // 22
}
```

Nested tuples:

```slynx
func main(): int {
    let nested = ((1, "jorge"), 1.0);
    let inner = nested.0;   // (1, "jorge")
    inner.1                 // "jorge"
}
```

Using aliases for tuples:

```slynx
alias Person = (str, int);

func new_person(name: str, age: int) -> Person {
    (name, age)
}
```

## Interaction

- The parser produces `ASTExpression::Tuple` for literals and `TupleAccess { tuple, index }` for access.
- The `.0`/`.1` indexing is handled in the parser's postfix chain.
- The HIR checks the tuple size on access (out-of-range index is an error).
- In the IR, tuples are lowered as structs with positional fields.

## Summary

| Aspect | Detail |
|--------|--------|
| Type | `(T1, T2, ...)` |
| Literal | `(v1, v2, ...)` |
| Access | `tuple.N` (integer index) |
| Empty tuple | `()` |
| Nesting | Allowed |
| Invalid index | Type error |
