# Extending monomorphization to other declarations

This guide explains how to add monomorphization for a new declaration kind, or
to complete one of the kinds that is still stubbed. It assumes you have read
[`architecture.md`](./architecture.md).

## What is implemented today

| Kind | Trigger | Status |
|---|---|---|
| Function | `HirExpressionKind::FunctionCall.generics` | Implemented |
| Struct (`object`) | `HirType::Reference` in an object literal / type position | Implemented |
| Component | `HirType::Reference` in a component expression / type position | Implemented |
| Enum | `HirType::Enum` (generic payload) in an enum expression / `matches` / type position | Implemented |
| Type alias (`alias`) | — | Not implemented (`unimplemented!`) |
| Stylesheet (`style`) | — | Not implemented (`unimplemented!`) |
| Static (`static`) | — | Not implemented (`unimplemented!`) |
| Object methods | — | Not implemented (specialized structs are created with no methods) |

The remaining kinds are rejected in `run` via `assert_no_generic_non_functions`.
To support one of them, follow the steps below.

## The core idea

Monomorphization is driven by **generic type information that survives HIR
building**. For functions this is the `generics: Vec<DedupPoolId<HirType>>`
field on `FunctionCall`; for structs and components it is a
`HirType::Reference { rf, generics }` produced by `HirNode::find_type` when the
source writes a generic application (`Option<int>`, `List<int>`); for enums it
is the `HirType::Enum` payload types, specialized by `resolve_enum_target` and
neutralized by `neutralize_generic_enums`.

> **If the HIR drops the generic arguments, monomorphization has nothing to
> work on.** The very first step for any new kind is to make the HIR builder
> record the type arguments that appear at use sites. See "Step 1" below.

## Step 1 — Make the HIR record generic applications

Before you can specialize anything you must be able to *see* the type
arguments. For structs and components this was done by:

- `HirNode::find_type` (`crates/hir/src/builders/mod.rs`): when a
  `Type::Plain(GenericIdentifier { generic, .. })` has a non-empty `generic`
  list and resolves to a struct or component, it now returns
  `HirType::Reference { rf: <template type>, generics: <resolved args> }`
  instead of the bare template type.
- Callers that previously matched on the raw type (object literals, child
  component signatures) now go through `view.dereference()` so a `Reference` is
  transparent.

For a *new* kind, find the analogous place. Examples:

- **Aliases** — an alias application `Vec<int>` currently resolves through the
  alias target and the args are lost; the alias's `ty` is substituted instead.
  You may want to keep a `Reference` to the alias declaration so the
  monomorphizer can specialize the alias target per argument list.
- **Styles** — `HirStyleUsage` already carries `params` (expressions), but the
  *stylesheet* arguments are concrete expressions, not types. If a stylesheet
  can be generic over types, add the type-argument list to `HirStyleUsage`.

## Step 2 — Create `structs.rs`-style module (or extend the existing one)

Create a new file `crates/monomorphizer/src/<kind>.rs` and declare it in
`lib.rs`:

```rust
mod <kind>;
```

Implement the specialization as an `impl Monomorphizer` block. Follow the recipe
from `architecture.md` § "The specialization recipe", reusing:

- `MonomorphizationKey` and the `cache` / `in_progress` / `dead_code` fields;
- `types::substitute_type` for type substitution;
- `types::mangle_name` for the mangled specialization name;
- the tree builders in `lib.rs` (`build_expression`, `build_statements`, …) to
  rebuild any expressions the declaration contains.

### Step 2a — Teach the shared skeleton about the new kind

`specialize` is generic over the declaration kind, so a new kind needs its
declaration types in `src/specialization.rs`:

- implement [`NamedDeclaration`](../src/specialization.rs) (gives `specialize`
  the template's name for diagnostics and mangling);
- implement [`SpecializableDeclaration`](../src/specialization.rs) — a
  `TypeDeclaration` impl (usually already present in `slynx_hir`) plus
  `from_erased`, which narrows the erased cache-hit id back to a typed
  `DeclarationId<Self>`.

Then write a `resolve_<kind>_target` that builds a
[`SpecializationDescriptor`](../src/specialization.rs) — typed template id,
concrete `args`, use-site `span`, and a `build` callback that creates the
specialized declaration under the mangled name — and hands it to `specialize`.
The skeleton performs the arity check, cache lookup/hit, cycle guard, mangling,
cache memoization, and the dead-template mark itself.

## Step 3 — Wire the trigger into `lib.rs`

The `run` driver must (a) rewrite the places that *use* the generic kind, and
(b) neutralize the generic templates.

### Trigger points (a)

- If the kind is used from **expressions**, add a check in the matching arm of
  `build_expression` (or `build_component_expression`). This is where
  `resolve_object_target` / `resolve_component_target` are called today.
- If the kind is used from **signatures** (function args/return, property
  types, variable types), add a pass that walks the type and resolves every
  `HirType::Reference` whose `generics` are concrete back into a specialization.
  Today this is `resolve_expression_type` in `lib.rs`, applied to non-generic
  function signatures and reused when building specialized struct/component
  types.
- If the kind is used from **top-level declarations** (e.g. a `static` whose
  type is `Option<int>`), extend `run` to rewrite those declarations too.

### Neutralization (b)

Write a `neutralize_generic_<kind>` method in the kind's module that mirrors the
existing four: collect the templates with the shared
[`generic_templates`](../src/lib.rs) scan (every `TypeDeclaration` whose
`generics()` is non-empty across the files), empty the declaration's body /
reset its type to `void_ty`, and record each one via
[`mark_dead`](../src/lib.rs), which erases the typed id to `AnyDeclarationId`
for the heterogenous dead-code set. Codegen skips dead declarations, so this is
what keeps `GenericParam`-typed data away from it. Call the new method from step
4 of `run` alongside `neutralize_generic_functions` / `_objects` /
`_components` / `_enums`.

## Step 4 — Naming and dedup

Decide how the new specialization is stored and keyed:

- **Key**: use `(AnyDeclarationId, SmallVec<[DedupPoolId<HirType>; 2]>)` —
  identical argument lists must map to one specialization. Nothing to do beyond
  implementing `SpecializableDeclaration`: the template's id is erased with
  `TypeDeclaration::as_any_id` when the key is built and narrowed back with
  `from_erased` on hits; `mark_dead` adds the template to `dead_code`.
- **Storage**: insert the specialized declaration into the same per-file
  `DeclarationsPool` as the template (e.g. `file.declarations.objects`), so
  codegen's `hoist_declarations` picks it up automatically. Give it a unique
  mangled name so it never collides with a user declaration or another
  specialization.
- **Methods/attached data**: if the specialized kind carries references to
  other declarations (struct methods point at `HirFunctionDeclaration`s),
  decide whether to copy them or specialize them too. Generic struct methods
  are currently **dropped** from specialized structs — the docs and this guide
  treat that as a known limitation.

## Step 5 — Tests

Every new kind should get, at minimum:

1. An example in `examples/generics/` that exercises the full pipeline
   (`slynx::compile_to_ir`), run by `tests/generics.rs`.
2. A unit test in `tests/monomorphizer.rs` asserting the **dedup** property
   (two identical use sites → exactly one specialization), like
   `deduplicates_identical_instantiations`.
3. A negative test for arity mismatch, like `rejects_wrong_generic_arity`.

## Known limitations (as of this milestone)

- Generic **struct methods** are not specialized; a specialized struct is
  created with an empty method table.
- Generic **object / component templates** are always neutralized (dead code),
  even if never instantiated.
- `unify_types` refuses to unify two *different* `GenericParam` ids, so a
  generic body must not mix two distinct type parameters in one binary
  operation.
- Explicit type arguments only; call-site type inference is out of scope.
- Type aliases, stylesheets, statics, and object methods are not implemented
  yet and hit `assert_no_generic_non_functions` (`unimplemented!`).