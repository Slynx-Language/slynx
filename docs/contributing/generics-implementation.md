# Generics Implementation

> Status: **implemented** — generic functions, generic objects (structs), generic
> components, and generic enums are monomorphized end-to-end. The language does
> **not** support generic type aliases or generic stylesheets (hard error), and
> call-site type inference is not implemented (explicit `<...>` arguments are
> required).
>
> This document describes how generics work across the pipeline today, the
> implemented monomorphizer design, and the known limits.

## 0. Scope

### Implemented

- Generic functions: `func identity<T>(x: T): T`
- Generic objects: `object Nullable<T> { value: T }`
- Generic components: `component List<T> { ... }`
- Generic enums: `enum Status<T> { Ok(T), Failed(T) }` (see the
  [monomorphizer extension guide](../../crates/monomorphizer/docs/extension-guide.md))
- Unconstrained type parameters only — `<T>` with no bounds, no defaults.
- Explicit use-site application: `func identity(x: T): T` called as
  `identity<i32>(x)`; also `obj`-style `Name<T>(...)` for components.
- **Monomorphization**: one specialized declaration per unique type-argument
  list, memoized, with concrete types substituted everywhere.

### Not implemented (known limits)

- Generic **type aliases** (`alias Foo<T> = ...`) → `unimplemented!()` in
  `assert_no_generic_non_functions` (`crates/monomorphizer/src/lib.rs`).
- Generic **stylesheets** (`stylesheet A<T>()`) → same `unimplemented!()`.
  Despite that, `examples/generics/generic_style.slx` compiles because the
  generic stylesheet is never referenced (enqueueing is lazy), so the
  `unimplemented!()` is never hit.
- **Type inference at call sites** (`identity(x)` without explicit `<...>`
  args). `examples/generics/inferences.slx` is marked `// xpass:` because it
  currently panic-compiles deep in codegen ("no entry found for key") instead of
  producing a clean error.
- **Generic object methods**: a specialized struct is created with an **empty
  method table**, so calls to methods defined on a generic struct are not
  supported on the specialized type.
- **Concepts / trait bounds** (`T: Concept`, `concept` keyword), **const
  generics**, **`extend<T> []T { ... }`**.

## 1. Pipeline

```
source → lexer → parser → AST → [HIR + type check] → monomorphizer → codegen → .sir
```

Type resolution and checking happen inside the HIR builders
(`crates/hir/src/builders/`); there is no separate `crates/checker` crate.
The monomorphizer runs after `SlynxHir::new` inside
`SlynxContext::build_hir` (`src/compilation_context/mod.rs`), returning the set
of dead declarations that the codegen skips. The pipeline ends at a textual
`.sir` IR; there is no JS backend in this repository.

## 2. How generics are represented

- At the call site, `identity<i32>` (or a use-site `Name<T>` that is followed by
  `(` or `{`) is parsed by `parse_type` into a `Type::Plain(GenericIdentifier)`
  (`crates/parser/src/types.rs`, `crates/parser/src/ast/types.rs`). The parser
  distinguishes generic application from the `<` operator via
  `Parser::is_generic` and `Parser::is_generic_application`.
- `split_type_params()` (`crates/parser/src/types.rs`) tucks a declaration-site
  `<T...>` off the function/object/component/enum name into the declaration's
  `type_params`, and `push_type_params()` interns them into the declaration
  scope.
- In the HIR, a declared parameter is `HirType::GenericParam { owner, index }`
  (`crates/hir/src/model/types.rs`). Because identifiers are interned into a
  single global pool, `GenericParam` is tagged with its owning declaration so
  two distinct `T` parameters never collide after dedup.
- `HirFunctionDeclaration` / the object/component/enum declarations carry their
  `type_params` into the HIR, and the HIR builders resolve a bare name matching
  a declared parameter to its `GenericParam` id (before module lookup), letting
  `func identity<T>(x: T): T` type-check and unify.

## 3. Monomorphizer

`crates/monomorphizer/src/`:

| File | Role |
|---|---|
| `lib.rs` | `Monomorphizer::resolve()` driver; `neutralize_generic_*` for each declaration kind; `assert_no_generic_non_functions()` — generic aliases/stylesheets are a hard error |
| `types.rs` | `Substitution`, `mangle_name()`, `substitute_type()`, `resolve_expression_type()` |
| `functions.rs` | generic function specialization |
| `structs.rs` | generic object (struct) specialization — **empty method table** on the specialized struct |
| `components.rs` | generic component specialization |
| `enums.rs` | generic enum specialization — `resolve_enum_target()`, `neutralize_generic_enums()` |

`resolve()` runs in this order:

1. Build a worklist of every generic declaration reachable from the AST.
2. For each unique `(declaration, concrete type-arg list)`:
   - compute a **mangled name** with `mangle_name()` — the template name
     followed by one `_<TypeName>_<hash>` segment per concrete argument, where
     the hash is structural over the `HirType` value (so names stay unique and
     human-readable);
   - clone the template declaration and build a `Substitution`
     (`index → DedupPoolId<HirType>`);
   - substitute every `GenericParam` leaf in signatures, field types, and nested
     calls with the concrete types via `substitute_type()`;
   - insert the specialized declaration into the same per-file pool, so codegen
     picks it up as an ordinary declaration.
3. `neutralize_generic_objects`, `neutralize_generic_components`, then
   `neutralize_generic_enums` — generic substitutable types get resolved to
   their specializations, and remaining generic leftovers are neutralized to a
   shared base (the "void" type) so codegen sees only concrete types.
4. `assert_no_generic_non_functions` rejects generic aliases and generic
   stylesheets with `unimplemented!()`.

The result is a fully concrete HIR plus the deadcode set
(`HashSet<AnyDeclarationId>`) passed to the codegen.

## 4. IR and codegen

- `IRType` (`crates/ir/src/types/irtype.rs`) is fully concrete; no generic IR
  types exist or are needed. By codegen time every generic reference has been
  replaced by a specialization.
- Codegen is keyed off HIR ids: `hoist_declarations`
  (`crates/codegen/src/lib.rs`) creates one IR function per HIR declaration, so
  specialized declarations under their mangled names become distinct IR
  functions automatically. `deadcode` marks unreachable generic templates so
  they are not hoisted.
- No codegen changes are needed when adding a new generic declaration kind.

## 5. Verification

- `tests/generics.rs` compiles every file in `examples/generics/` and enforces
  the `// xfail:` (must fail) and `// xpass:` (known missing validation)
  marker conventions. `tests/monomorphizer.rs` covers the pass directly.
- Dedupe is covered by `examples/generics/dedup.slx` (two call sites, one
  specialization).
- Known-boilerplate examples exist for functions over arrays/vectors
  (`generic_over_array.slx`, `generic_over_vector.slx`), multi-param
  (`multi_param.slx`), nesting (`nested.slx`), and nullable interplay
  (`nullable_*.slx`).
- Run `cargo test --workspace` after changes.

## 6. Key file map

| Concern | File |
|---|---|
| AST generic identifier / declaration `type_params` | `crates/parser/src/ast/types.rs`, `crates/parser/src/ast/declarations.rs` |
| Type parsing + `is_generic` / `is_generic_application` / `split_type_params` | `crates/parser/src/types.rs` |
| Expr parsing (calls/components with `<...>`) | `crates/parser/src/expr.rs`, `crates/parser/src/functions.rs` |
| `HirType::GenericParam` | `crates/hir/src/model/types.rs` |
| HIR declaration kinds + `type_params` | `crates/hir/src/model/declarations.rs` |
| Type resolution / signature | `crates/hir/src/builders/mod.rs` |
| Body building / type checking | `crates/hir/src/builders/expression/` |
| Monomorphizer driver | `crates/monomorphizer/src/lib.rs` |
| Substitution + mangling | `crates/monomorphizer/src/types.rs` |
| Specialization per kind | `crates/monomorphizer/src/{functions,structs,components,enums}.rs` |
| Pipeline wiring | `src/compilation_context/mod.rs` |
| Codegen declaration hoisting | `crates/codegen/src/lib.rs` |
| Example/test harness | `examples/generics/`, `tests/generics.rs`, `tests/monomorphizer.rs`, `tests/enums.rs` |