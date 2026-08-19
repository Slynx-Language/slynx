# Monomorphizer architecture

> Status: **implemented** for functions, structs (objects), and components.
> Aliases, styles, statics, and object methods are not implemented yet — see
> [`extension-guide.md`](./extension-guide.md).

## Goal

Monomorphization turns *generic* declarations into *concrete* ones. A generic
declaration has a list of type parameters (`<T>`); every use site that applies
concrete type arguments (`identity<int>(x)`, `Option<int>(value: 42)`) gets a
dedicated, non-generic copy of the declaration with the parameters substituted,
plus a rewritten reference to that copy.

The pass runs on the finished HIR (after `SlynxHir::new`) and before codegen.
Codegen then sees **no** `HirType::GenericParam` and no generic declaration at
all.

## Module layout

Each source file in `crates/monomorphizer/src/` owns one piece of the pass:

| File | Responsibility |
|---|---|
| [`lib.rs`](../src/lib.rs) | Driver. Owns the `Monomorphizer` state (cache, in-progress set, dead-code set), the public [`resolve`](../src/lib.rs) entry point, the `run` driver loop, and the declaration-agnostic expression/statement **tree builders** (`build_expression`, `build_statements`, `build_component_expression`, …). |
| [`types.rs`](../src/types.rs) | Type machinery shared by every declaration kind: the `Substitution` map, [`mangle_name`](../src/types.rs), and [`substitute_type`](../src/types.rs) (walks an `HirType` and replaces `GenericParam` leaves). |
| [`functions.rs`](../src/functions.rs) | Function specialization: [`resolve_function_target`](../src/functions.rs) creates/retrieves the concrete copy of a generic function for one set of type arguments, and [`function_return_type`](../src/functions.rs) reads a specialization's return type. |
| [`structs.rs`](../src/structs.rs) | Struct (object) specialization: [`resolve_object_target`](../src/structs.rs) creates/retrieves a concrete struct type + `HirObjectDeclaration` for one instantiation, and rewrites generic object usage in expression/signature positions. |
| [`components.rs`](../src/components.rs) | Component specialization: [`resolve_component_target`](../src/components.rs) creates/retrieves a concrete component type + `HirComponentDeclaration` (including rebuilt property defaults and child tree), and rewrites generic component usage. |

### Why split per declaration kind?

Monomorphization of each declaration kind is largely independent:

- the **trigger** is different (a `FunctionCall.generics` field vs. a
  `HirType::Reference` in an object literal vs. a `HirType::Reference` in a
  component expression);
- the **artifact** is different (a `HirFunctionDeclaration` with statements, a
  `HirObjectDeclaration` with a struct type, a `HirComponentDeclaration` with
  property members);
- the **dedup cache** is keyed by the same `(template, type_args)` shape but
  stored under the declaration's `AnyDeclarationId`.

Keeping one file per kind means adding a new kind (aliases, styles, …) touches
`lib.rs` only to add a `mod` line and a dispatch in `run`; the rest of the
changes live in the new file.

## The `Monomorphizer` state

```rust
pub struct Monomorphizer {
    cache: DashMap<MonomorphizationKey, AnyDeclarationId>,
    in_progress: HashSet<MonomorphizationKey>,
    dead_code: HashSet<AnyDeclarationId>,
}
```

- `cache` — memo table mapping `(template declaration id, type-argument list)`
  to the already-generated specialization. Two use sites with the same args
  yield **one** specialization (deduplication).
- `in_progress` — instantiations currently being generated. If a key re-enters
  itself while still in progress, the instantiation is non-terminating and we
  raise `HIRError::cyclic_monomorphization`.
- `dead_code` — the set of generic templates that were neutralized. Returned by
  [`resolve`](../src/lib.rs) and consumed by codegen (`generate` skips
  declarations in this set).

`MonomorphizationKey = (AnyDeclarationId, SmallVec<[DedupPoolId<HirType>; 2]>)`.

## The `run` driver

[`Monomorphizer::run`](../src/lib.rs) drives the pass in four steps:

1. **Rewrite non-generic function bodies.** Every generic call site
   (`identity<int>(x)`) is resolved; the returned specialization replaces the
   call target. Specializations discovered here may contain further generic
   calls, resolved recursively through `resolve_function_target`.
2. **Rewrite non-generic component members.** Property default values and child
   component trees may contain generic object/component usage.
3. **Rewrite non-generic function signatures.** A signature such as
   `func f(o: Option<int>)` contains a `HirType::Reference` to a generic
   template; references are resolved to the concrete specialization.
4. **Neutralize generic templates.** Every remaining generic declaration gets
   an empty body / neutral type and is inserted into `dead_code`, so codegen
   never sees a `GenericParam`-typed signature.

Steps 1–3 share the same tree builders from `lib.rs`; they differ only in the
node they rewrite (a `HirFunctionDeclaration` vs a `HirComponentDeclaration`
vs a `HirType`).

## The specialization recipe

Every `resolve_*_target` follows the same recipe:

1. Find the **template** declaration (by symbol name for objects/components, by
   `DeclarationId` for functions).
2. **Arity check**: the template's type-parameter count must equal the number of
   supplied type arguments, otherwise `HIRError::generic_arity_mismatch`.
3. Look up the `MonomorphizationKey` in `cache` → return the cached
   specialization if present.
4. Guard against `in_progress` re-entry (cyclic instantiation).
5. Build the `Substitution` (`parameter index → concrete type`) and mangle the
   specialization name (see below).
6. Create the specialized declaration **with an empty body** and insert it into
   the pool, then **cache it before filling the body**. Caching first makes
   recursion work: a specialization that refers to itself resolves to itself.
7. Fill the body (substitute + rebuild statements/expressions/members).
8. Mark the template as dead.

## Mangling

`mangle_name` produces `<template-name>_<arg0-name>_<arg0-hash>_<arg1-name>…`.
Each type argument contributes one `_<name>_<4-hex-hash>` segment, so the
specialization name is unique per type-argument list while staying
human-readable. The hash is a structural `DefaultHasher` over the `HirType`
value.

## Type substitution

`substitute_type` walks an `HirType` and replaces every
`HirType::GenericParam { index, .. }` whose `index` appears in the
`Substitution`. It recurses into `Array`, `Vector`, `Function`, `Tuple`, and
`Reference` (for a `Reference`, only generic slots whose current value is a
substituted `GenericParam` are replaced — null padding slots are kept).

Specialization of a generic *type* (`Option<int>`) is a second, separate step:
`resolve_object_target` / `resolve_component_target` take the `Reference`'s
concrete generic arguments and produce a **brand-new** struct/component type
whose fields/properties are `substitute_type`-substituted.

## Invariants

- After `resolve` returns, no reachable declaration has a
  `HirType::GenericParam` anywhere in a signature, body, or expression type.
- Every generic template (functions, objects, components) is in `dead_code`.
- No two specializations share a name (mangling) and no specialization collides
  with a user declaration (specializations are pooled next to templates).