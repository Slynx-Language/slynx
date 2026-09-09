# Current Language Surface

This document describes the language surface that is currently supported on the
`main` branch of `Slynx-Language/slynx`.

It is intentionally scoped to what the repository can already lex, parse,
lower into HIR, and type-check today. It should be safe to reuse this document
on the landing page or docs site without marketing unfinished features as
already available.

For feature-specific documentation, see [`docs/features/`](features/).

## Status

This is a grounded overview of the current language surface.

It does **not** try to document:

- slot syntax as implemented code;
- reactive graph lowering as implemented code;
- a stable public CLI workflow;
- a stable backend or IR contract beyond the current debug-style dumps.

For design-only topics, see:

- [docs/goals/component-slots.md](goals/component-slots.md)
- [crates/ir/docs/reactive-graph-generation.md](../crates/ir/docs/reactive-graph-generation.md)
- [crates/ir/docs/linerize-the-graph.md](../crates/ir/docs/linerize-the-graph.md)

## Top-Level Declarations

The current parser accepts nine top-level declaration kinds:

- `object`
- `component`
- `func`
- `alias`
- `enum`
- `static`
- `stylesheet`
- `import`
- `extern` (blocks)

Example:

```slynx
import std using Color;

object Person {
    name: str,
    age: int,
}

alias PersonRef = Person;

enum Status {
    Idle,
    Busy,
    Done,
}

static MAX: int = 100;

component Profile {
    Text {
        text: "profile"
    }
}

func main(): Component {
    Profile {}
}
```

Relevant feature docs:

- [features/enums.md](features/enums.md) — enum declarations and variants
- [features/static-variables.md](features/static-variables.md) — static values
- [features/stylesheet-declarations.md](features/stylesheet-declarations.md) — stylesheets
- [features/imports.md](features/imports.md) — module imports
- [features/externs.md](features/externs.md) — extern blocks
- [features/visibility-modifiers.md](features/visibility-modifiers.md) — `pub` and friends
- [features/attributes.md](features/attributes.md) — `@name(...)` attributes

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
- the last expression in a block can act as the implicit return value
  (see [features/implicit-return.md](features/implicit-return.md));
- a non-`void` function should still end in a return-producing expression;
- explicit early returns use `return` (see
  [features/return-statements.md](features/return-statements.md)).

## Statements

Inside function bodies, the current frontend supports:

- immutable `let`;
- mutable `let mut`;
- assignment;
- `while`;
- `return`;
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
- field access expressions such as `person.age`;
- dereferences such as `*ptr` (see [features/dereference.md](features/dereference.md)).

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

### Return

`return` terminates a function early, optionally with a value. See
[features/return-statements.md](features/return-statements.md).

```slynx
func mod(a: int, b: int): int {
    if a < b {
        return a;
    }
    return a % b;
}
```

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

Tuple access via dot-index is part of the frontend surface:

```slynx
func pair(): int {
    let pair = (1, "ok");
    pair.0
}
```

Tuple access is also implemented and tested (see `examples/tupleAccess.syx`).

See [features/tuples.md](features/tuples.md) for the full tuple documentation.

### Nullable Types

Any type can be made nullable with the `?` suffix. See
[features/nullable-types.md](features/nullable-types.md).

```slynx
let a: int? = null;
let b: int? = 15;
let v: ( []int )? = null;
```

The `null` literal is described in
[features/null-literal.md](features/null-literal.md).

### Reference Types

Types `&T` (immutable reference) and `&mut T` (mutable reference) are
implemented in the type system. See
[features/move-semantics.md](features/move-semantics.md) and
[features/reference-expressions.md](features/reference-expressions.md).

```slynx
func get(&self): int { ... }
func main(): &int { return &x; }
```

### Self Type

Inside object methods, `Self` refers to the enclosing object type. See
[features/self-type.md](features/self-type.md).

### Generic Types

Functions, objects, components, enums, and stylesheets can be generic. See
[features/generics.md](features/generics.md) and:

- [features/generic-functions.md](features/generic-functions.md)
- [features/generic-structs.md](features/generic-structs.md)
- [features/generic-components.md](features/generic-components.md)
- [features/generic-enums.md](features/generic-enums.md)
- [features/generic-stylesheets.md](features/generic-stylesheets.md)

## Expressions

The current surface includes:

- integer literals (including `0x`, `0b`, `0o` — see
  [features/number-systems.md](features/number-systems.md));
- float literals;
- string literals;
- boolean literals;
- `null` (see [features/null-literal.md](features/null-literal.md));
- identifiers;
- function calls;
- tuple literals;
- object expressions;
- component expressions;
- field access;
- binary expressions;
- `if` expressions;
- `matches` expressions (see
  [features/matches-expression.md](features/matches-expression.md));
- array/vector literals and indexing (see
  [features/arrays-vectors-slices.md](features/arrays-vectors-slices.md));
- slicing (see [features/slicing.md](features/slicing.md));
- references `&expr` and `&mut expr` (see
  [features/reference-expressions.md](features/reference-expressions.md));
- dereference `*expr` (see [features/dereference.md](features/dereference.md)).

### Literals

```slynx
let i = 10;
let f = 1.5;
let s = "hello";
let ok = true;
let n = null;
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

Note: `>>` (right shift) is not currently a token in the lexer. It was removed
because of a conflict with generic syntax: `A<B>>` in expression context was
being parsed as a `>>`. Generic instantiation uses `>` instead.

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

### Matches

The `matches` expression checks whether an enum value corresponds to a given
variant, optionally comparing associated values. See
[features/matches-expression.md](features/matches-expression.md).

```slynx
enum Status { Idle, Busy, Done }

func is_done(s: Status): bool -> s matches Done;
```

### Arrays, Vectors, and Slicing

Arrays use `[N]T`, vectors use `[]T`. Literals use `[...]` for arrays and
`{...}` for vectors. Indexing and slicing read from these collections. See
[features/arrays-vectors-slices.md](features/arrays-vectors-slices.md) and
[features/slicing.md](features/slicing.md).

```slynx
let a: [4]int = [1, 2, 3, 4];
let b: []int = {1, 2, 3};
let all = a[:];
let mid = a[1:3];
let first = a[0];
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

### Object Methods

Objects can declare methods with `func` inside their body. Methods may take
`self`, `&self`, or `&mut self` as the receiver, or be type-associated
(constructors/factories). See
[features/struct-methods.md](features/struct-methods.md).

```slynx
pub object Counter {
    count: int,

    func new(): Self { Self(count: 0) }
    func increment(&mut self, amount: int): void {
        self.count = self.count + amount;
    }
    func getValue(&self): int {
        self.count
    }
}
```

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
- default prop values;
- visibility modifiers `pub`, `pub(parent)`, `pub(child)` (see
  [features/visibility-modifiers.md](features/visibility-modifiers.md)).

Example:

```slynx
component Counter {
    pub prop count = 0;

    Text {
        text: count
    }
}
```

### Stylesheets

Components can receive style via the `style:` property. Stylesheets are
declared with `stylesheet` and can use inheritance (`uses`) and state blocks.
See:

- [features/stylesheet-declarations.md](features/stylesheet-declarations.md)
- [features/style-inheritance.md](features/style-inheritance.md)
- [features/style-state-blocks.md](features/style-state-blocks.md)

```slynx
stylesheet Maria(color: int) {
    styles {
        default {
            backgroundColor: color,
        }
    }
}

component App {
    Div {
        style: Maria(0xff0000)
    }
}
```

### Special Case: `children`

The parser has a dedicated `prop children;` path that currently maps to a
vector-of-components style shape internally.

That is parser/frontend behavior today, but the full slot/child model is still
being designed separately.

## Enums

Enums define union types with variants that may carry associated values. See
[features/enums.md](features/enums.md).

```slynx
enum Option<T> {
    Some(T),
    None,
}

let opt: Option<int> = Option.Some(42);
```

## Comments

The lexer supports common line comments and block comments:

```slynx
// this is a line comment
/* this is a block comment */
func main(): int -> 0;
```

See [features/block-comments.md](features/block-comments.md).

## Generics

Generics are implemented via monomorphization for functions, objects,
components, enums, and stylesheets. See
[features/generics.md](features/generics.md).

## Ownership and Move Semantics

Values are moved by default; primitives (`int`, `float`, `bool`, `str`) are
Copy. The ownership analysis checks use-after-move, conflicting borrows, and
mutability rules. See
[features/move-semantics.md](features/move-semantics.md).

## Current Non-Goals Of This Document

These topics are still being discussed or implemented and should not be read as
finished language surface:

- slot syntax as implemented code;
- reactive graph lowering;
- final child representation in the IR;
- generic-specialized component lowering;
- finalized backend semantics;
- a polished CLI workflow for dump generation;
- `&atomic T` reference type;
- type inference for generic call sites;
- const generics, trait bounds, and interface/trait system.

## Practical Reading

If you want to see real examples that match the current repository syntax,
start here:

- [examples/](../examples/)
- [examples/component.syx](../examples/component.syx)
- [examples/variables.syx](../examples/variables.syx)
- [examples/booleans.syx](../examples/booleans.syx)
- [examples/objects.syx](../examples/objects.syx)
- [examples/ifExpression.syx](../examples/ifExpression.syx)
- [examples/while.syx](../examples/while.syx)
- [examples/tupleAccess.syx](../examples/tupleAccess.syx)
- [examples/nullables.syx](../examples/nullables.syx)
- [examples/numberSystems.syx](../examples/numberSystems.syx)
- [examples/arrays.syx](../examples/arrays.syx)
- [examples/commonComments.syx](../examples/commonComments.syx)