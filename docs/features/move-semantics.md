# Move Semantics

Move semantics on the codebase works such as rust's move semantics. Until now, it contains 2 types of references: &T and &mut T.

Ideally, the idea is that every value should be used once, and then discarded, then the move helps us to find who's got the ownership of that value and then track where and when it should be discarded.
No drops are idealized on the codebase since the main goal is to NOT rely on top of RAII.
For evicting moving everything and having to copy a lot on the codebase, then its used reference types, which are cheaply copied and do not require any memory allocation.
## Main Idea
The idea is pretty straightforward, a type `&T`, means that &T can be used to read values of that given type T, but without any kind of ownership, nor mutability.
In counterpart to it, a type `&mut T` means that this T, is being passed as a reference such as `&T` and got's both read and write access to the value. Due to this, the same rules of the rust borrow checker are applied here.

## Rules 
Only one `&mut T` can exist at a time, and it must be unique, and no other `&T` might exist with a `&mut T` exists. This is mainly to avoid data races and undefined behavior via aliasing.
A bunch of `&T` types can exists at the same time, but as long as there is no `&mut T` in the mix, then the borrow checker will not complain.

### Extra
Another variant is idealized to help things on the codebase, in this case it's the `&atomic`, the main reason for this is that any `&atomic` can be used to read and write values of that given type T, the same way it does with `&mut T`, but the difference is that this write and read is atomic, so it's safe to use in concurrent environments, so, even though it might write, it's possible to have a lot of `&atomic` types in the codebase.
Note that this `&atomic` type doesn't define that that given type T uses atomic operations under the hood, it means that when writing/reading its contents, it 100% safe in environments of threaded execution. An &atomic T might be able to be atomic even though it uses a mutex or a spinlock under the hood, because even though it's not using atomic operations directly, it's still safe in threaded environments.
# Move Semantics

Slynx uses an ownership and move semantics model inspired by Rust's ownership system.

The main goal of move semantics is to make ownership explicit and allow the compiler to track which part of the program currently owns a value and where that value can be used.

A value is normally **moved** when it is transferred to another owner. After a value has been moved, the previous owner can no longer use that value.

The language does not rely on traditional RAII-based destruction as its fundamental memory-management model. Instead, ownership and lifetime information are tracked explicitly by the compiler.

To avoid unnecessarily moving or copying large values, Slynx provides reference types. References do not transfer ownership and are intended to be cheap to pass around.

## Reference Types

Slynx currently provides three relevant reference forms:

* `&T` — shared, immutable reference
* `&mut T` — unique, mutable reference
* `&atomic T` — shared, atomically synchronized reference

The first two follow the same fundamental aliasing rules as Rust's shared and mutable references.

### `&T`

A value of type `&T` is a shared reference to a value of type `T`.

It:

* does not own the referenced value;
* provides read-only access to the value;
* may coexist with other `&T` references;
* cannot be used to mutate the referenced value.

For example:

```slynx
let value: T = ...;
let reference: &T = &value;
```

Creating or passing an `&T` does not transfer ownership of `value`.

Multiple `&T` references to the same value are allowed:

```text
        ┌─────────┐
        │    T    │
        └─────────┘
          ↑  ↑  ↑
          │  │  │
         &T &T &T
```

As long as the references remain shared and immutable, they may coexist.

## `&mut T`

A value of type `&mut T` is a unique mutable reference to a value of type `T`.

It:

* does not own the referenced value;
* provides both read and write access;
* must be unique while it exists;
* cannot coexist with any `&T` reference to the same value;
* cannot coexist with another `&mut T` reference to the same value.

For example:

```slynx
let mut value: T = ...;
let reference: &mut T = &mut value;
```

While `reference` exists, no other shared or mutable reference to the same value may be used.

The fundamental rule is:

> Either there can be multiple shared `&T` references, or there can be exactly one `&mut T` reference, but never both at the same time.

Conceptually:

```text
Multiple readers:

        ┌─────────┐
        │    T    │
        └─────────┘
          ↑  ↑  ↑
          │  │  │
         &T &T &T


Exclusive writer:

        ┌─────────┐
        │    T    │
        └─────────┘
             ↑
             │
           &mut T
```

These rules prevent invalid aliasing and data races caused by simultaneous mutable and immutable access.

## Ownership and Moves

References do not transfer ownership. Moving a value does.

For example:

```slynx
let a: T = ...;
let b: T = a;
```

The assignment transfers ownership of the value from `a` to `b`.

After the move, `a` is no longer a valid owner of that value and cannot be used as if it still owned it.

Conceptually:

```text
Before:

a ─────► T


After:

a       b ─────► T
                 ownership
```

The compiler tracks these ownership transfers to determine where a value may be accessed.

The exact point at which a value becomes unusable after a move is therefore part of the borrow checker's responsibility.

## Why References Exist

Moving large values whenever they are passed between functions or stored in other structures could require unnecessary copies or data movement.

References provide an alternative:

```slynx
let value: T = ...;

foo(&value);
```

Instead of transferring ownership of `value`, `foo` receives a reference to it.

References are intended to be cheaply copied and do not require allocating a new copy of the referenced value.

## `&atomic T`

Slynx also is idealized to provide an `&atomic T` reference type for values that need to be accessed concurrently.

Unlike `&mut T`, multiple `&atomic T` references may coexist.

An `&atomic T` provides both read and write access to the referenced value, but those accesses are required to be synchronized so that concurrent access is safe.

For example:

```text
              ┌─────────┐
              │    T    │
              └─────────┘
               ↑   ↑   ↑
               │   │   │
            &atomic T
```

Multiple `&atomic T` references may therefore refer to the same value concurrently.

### Atomic Does Not Mean Hardware Atomic Instructions

The name `atomic` describes the **synchronization guarantees provided by the reference**, not necessarily the implementation mechanism used to achieve them.

An `&atomic T` may use hardware atomic instructions when appropriate, but it may also be implemented using mechanisms such as:

* mutexes;
* spinlocks;
* other synchronization primitives.

The important property is that concurrent reads and writes through `&atomic T` are synchronized and do not introduce data races.

Therefore:

> `&atomic T` means that access to `T` is safe for concurrent use; it does not necessarily mean that `T` itself is implemented using CPU atomic instructions.

## Interaction Between Reference Types

The borrow checker must distinguish between ordinary references and atomic references.

`&T` and `&mut T` follow exclusive-access rules:

* many `&T` references may coexist;
* one `&mut T` may exist exclusively;
* `&T` and `&mut T` cannot refer to the same value at the same time.

`&atomic T` is different because its purpose is to permit concurrent access.

Therefore, `&atomic T` references may coexist with other `&atomic T` references.

The exact interaction between `&atomic T` and ordinary `&T` / `&mut T` references must be defined by the borrow checker rules for the language. In particular, the language must specify whether an ordinary reference may coexist with an `&atomic T` reference to the same value.

## Summary

Slynx's ownership model can be summarized as:

| Type        | Ownership  | Read | Write           | Multiple references |
| ----------- | ---------- | ---- | --------------- | ------------------- |
| `T`         | Owns value | Yes  | Yes, if mutable | N/A                 |
| `&T`        | No         | Yes  | No              | Yes                 |
| `&mut T`    | No         | Yes  | Yes             | No                  |
| `&atomic T` | No         | Yes  | Yes             | Yes                 |

The central invariant for ordinary references is:

> Multiple readers are allowed, but mutable access must be exclusive.

The `&atomic T` type provides a separate mechanism for shared concurrent access where reads and writes must remain synchronized.
