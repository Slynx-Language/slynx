# Static Variables

## Overview

Static variables are variables with a static lifetime: they are initialized at program creation and remain alive until the program terminates. They cannot be mutated through ordinary mutable access, as this could introduce data races and require synchronization between threads.

## Main Idea

A `static` declaration associates a name with a type and a value in global scope. The value is initialized once and lives forever.

```slynx
static NAME: Type = expr;
```

If mutation is needed, it must go through mechanisms with synchronization guarantees — such as atomic operations or lock-free.

```slynx
static someValue: AtomicUint8 = AtomicUint8.new();

func main(): void {
    let value = someValue.fetch_add(1);
}
```

The mutation in the example is allowed because `fetch_add` is an atomic operation on the value, not ordinary mutable access.

## Syntax

```slynx
static NAME: Type = expr;
```

In `extern` blocks, initialization is omitted (the value comes from the runtime/host):

```slynx
extern {
    static PI: f64;
    static window: Window;
}
```

## Rules

- The type is required in the declaration.
- Static variables can be declared in `extern` blocks (where the value expression is omitted) or with a value in source.
- Static variables **cannot** be mutated through ordinary mutable access.
- To mutate, a mechanism with synchronization guarantees (atomic, lock-free) is required.
- Static variables **cannot** have mutable references.
- Static variables **cannot** be moved (their lifetime is tied to the entire program).
- Beyond lifetime and access restrictions, they behave like other variables.
- They support `pub` for export (see [visibility-modifiers.md](visibility-modifiers.md)).

## Examples

```slynx
static MAX: int = 100;

static COUNTER: AtomicUint32 = AtomicUint32.new();

func main(): void {
    let current = COUNTER.fetch_add(1);
    let m = MAX;
}
```

In extern:

```slynx
extern {
    static appName: str;
    static document: Document;
}

func main(): void {
    let name = appName;
    let d = document;
}
```

## Interaction

- Parser: `parse_static` in `crates/parser/src/declarations.rs`.
- HIR: `HirDeclarationKind::Static`.
- IR: static values become `GlobalValue` with `ZeroInit` (or reference `GlobalExtern` in the extern case), accessed via `Opcode::Global` / `Opcode::GlobalExtern`.

## Summary

| Aspect | Detail |
|---------|---------|
| Lifetime | Entire program |
| Ordinary mutation | Prohibited |
| Atomic/lock-free mutation | Allowed |
| Mutable references | Prohibited |
| Move | Prohibited |
| Extern | Allowed (no value in source) |
