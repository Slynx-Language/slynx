# Current Language Surface

This document describes the language surface that is currently supported on the
`main` branch of `Slynx-Language/slynx`.

It is intentionally scoped to what the repository can already lex, parse,
lower into HIR, and type-check today. It should be safe to reuse this document
on the landing page or docs site without marketing unfinished features as
already available.

## Status

This is a grounded overview of the current language surface.

It does **not** try to document:

- slot syntax as implemented code;
- reactive graph lowering as implemented code;
- generics as a finished language feature;
- tuple access or tuple destructuring as implemented code;
- a stable public CLI workflow;
- a stable backend or IR contract beyond the current debug-style dumps.

For design-only topics, see:

- [docs/component-slots.md](component-slots.md)
- [middleend/docs/reactive-graph-generation.md](../middleend/docs/reactive-graph-generation.md)
- [middleend/docs/linerize-the-graph.md](../middleend/docs/linerize-the-graph.md)

## Top-Level Declarations

The current parser accepts four top-level declaration kinds:

- `object`
- `component`
- `func`
- `alias`

Example:

```slynx
object Person {
    name: str,
    age: int,
}

alias PersonRef = Person;

component Profile {
    Text {
        text: "profile"
    }
}

func main(): Component {
    Profile {}
}
```

## Functions

Functions use the `func` keyword.

They currently support:

- typed arguments;
- an explicit return type;
- a block body;
- or a short arrow body.

### Block Body

```slynx
func add(a: int, b: int): int {
    let total = a + b;
    total
}
```

### Arrow Body

```slynx
func add(a: int, b: int): int -> a + b;
```

### Notes

- the return type is required in the current syntax;
- the last expression in a block can act as the implicit return value;
- a non-`void` function should still end in a return-producing expression.

## Statements

Inside function bodies, the current frontend supports:

- immutable `let`;
- mutable `let mut`;
- assignment;
- `while`;
- expression statements.

Example:

```slynx
func main(): int {
    let value = 1;
    let mut total: int = 2;
    total = value + total;
    total
}
```

### Assignment

Assignments currently work with:

- identifiers;
- field access expressions such as `person.age`.

Example:

```slynx
func main(): int {
    let mut count = 0;
    count = count + 1;
    count
}
```

### While

`while` is now part of the grounded frontend surface on `main`.

Example:

```slynx
func main(): void {
    let mut x = 0;
    while x < 10 {
        x = x + 1;
    }
}
```

Notes:

- the condition is type-checked as `bool`;
- statements inside the body are also type-checked;
- this document only claims the current frontend support, not a finalized
  runtime/backend contract.

## Types, Aliases, and Tuples

The current surface includes built-in types such as `int`, `float`, `bool`,
`str`, `void`, and `Component`, plus named object/component types declared in
source.

### Aliases

Top-level aliases use `alias`:

```slynx
object Person {
    age: int,
}

alias PersonAlias = Person;

func main(): PersonAlias {
    Person(age: 22)
}
```

This is already part of the parser/HIR/checker pipeline on `main`.

### Tuple Types

Tuple types use parentheses:

```slynx
func pair(): (int, str) {
    (1, "ok")
}
```

The empty tuple type is also recognized as `()`.

### Tuple Literals

Tuple literals are currently parsed, lowered, and type-checked:

```slynx
func pair(): ((int, str), float) {
    ((1, "jorge"), 1.0)
}
```

Important limitation:

- tuple access such as `value.0` is **not** implemented as finished behavior on
  `main` yet;
- tuple destructuring should also be treated as unfinished.

## Expressions

The current surface includes:

- integer literals;
- float literals;
- string literals;
- boolean literals;
- identifiers;
- function calls;
- tuple literals;
- object expressions;
- component expressions;
- field access;
- binary expressions;
- `if` expressions.

### Literals

```slynx
let i = 10;
let f = 1.5;
let s = "hello";
let ok = true;
```

Numeric separators are also accepted in numeric literals today, as long as the
placement is valid for the lexer.

### Function Calls

Function calls use parentheses and accept zero or more arguments.

```slynx
func ping(): int -> 0;

func main(): int {
    ping();
    0
}
```

### Binary Expressions

The current parser supports arithmetic, comparison, logical, and bitwise binary
operators.

Examples already covered by the codebase include:

- `+`
- `-`
- `*`
- `/`
- `==`
- `>`
- `>=`
- `<`
- `<=`
- `&&`
- `||`
- `&`
- `|`
- `^`
- `<<`
- `>>`

Example:

```slynx
func bigger(a: int, b: int): bool -> a > b && a >= b;
```

### If Expressions

`if` currently behaves as an expression.

Example:

```slynx
func main(): int {
    let x = if 1 > 9 {
        1
    } else {
        3
    };

    x
}
```

## Objects

Objects are declared with named fields.

Object expressions currently use parentheses with named arguments:

```slynx
object Person {
    name: str,
    age: int,
}

func main(): int {
    let mut person = Person(name: "John", age: 10);
    person.age = 55;
    person.age
}
```

### Notes

- object fields are currently separated by commas in declarations;
- object construction uses named fields;
- field access uses dot syntax.

## Components

Components are declared with the `component` keyword and use brace-based
construction.

Example:

```slynx
component Header {
    Div {
        Text {
            text: "Header"
        }
    }
}

func main(): Component {
    Header {}
}
```

### Component Members

The current parser already understands:

- child components inside a component body;
- `prop` declarations;
- `pub prop` declarations;
- default prop values.

Example:

```slynx
component Counter {
    pub prop count = 0;

    Text {
        text: count
    }
}
```

### Special Case: `children`

The parser has a dedicated `prop children;` path that currently maps to a
vector-of-components style shape internally.

That is parser/frontend behavior today, but the full slot/child model is still
being designed separately.

## Comments

The lexer currently supports common line comments:

```slynx
// this is a comment
func main(): int -> 0;
```

## Current Non-Goals Of This Document

These topics are still being discussed or implemented and should not be read as
finished language surface:

- slot syntax as implemented code;
- tuple access or tuple destructuring;
- final child representation in the IR;
- reactive graph lowering;
- generic-specialized component lowering;
- finalized backend semantics;
- a polished CLI workflow for dump generation.

## Practical Reading

If you want to see real examples that match the current repository syntax,
start here:

- [slynx/component.slynx](../slynx/component.slynx)
- [slynx/variables.slynx](../slynx/variables.slynx)
- [slynx/functioncall.slynx](../slynx/functioncall.slynx)
- [slynx/objects.slynx](../slynx/objects.slynx)
- [slynx/If.slynx](../slynx/If.slynx)
- [slynx/while.slynx](../slynx/while.slynx)
- [slynx/tuplas.sly](../slynx/tuplas.sly)
