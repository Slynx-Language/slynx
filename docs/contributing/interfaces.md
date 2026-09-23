# Interfaces

Status: **planned** (design decided; not yet implemented). This document is
the implementation contract for the short/mid-run "Java-like" interfaces
feature. It records the scope decisions, the design (all reuse of existing
seams, no new subsystems), the filtered task list, and the gates.

---

## Overview

An *interface* is a named set of method signatures that a concrete type may
promise to provide through an `impl` block. A generic function may bound a type
parameter with `where T: Interface` and call the interface's methods on it; the
call is resolved **statically** — at monomorphization, against the impl for the
concrete type.

```slynx
interface Stringifiable {
    func stringify(&self) -> str
    func length(&self) -> int
}

impl Stringifiable for Person {
    func stringify(&self) -> str { self.name }
    func length(&self) -> int { self.name.length() }
}

impl<T> Stringifiable for Option<T> {
    func stringify(&self) -> str { "some" }
    func length(&self) -> int { 1 }
}

func describe<T>(x: T) -> str where T: Stringifiable {
    x.stringify()
}
```

Long-run, interfaces are idealized to be Rust/Swift-like (associated types,
`dyn` dispatch, supertraits). Short/mid-run they are **Java-like**: nominal
signatures, static dispatch, no trait objects, no associated types. Everything
beyond the v1 slice is deferred explicitly (section "Deferred"), so this
feature lands without rebuilding the compiler.

## Scope decisions (v1)

| Decision | Choice |
|----------|--------|
| Keyword | `interface` (`concept` stays reserved, unused) |
| Generic impls | in v1 — `impl<T> I for Wrapper<T>` matched against concrete self-types |
| Where bounds attach | generic functions **and** object methods |
| Invocation form | explicit generic args stay (`f<int>(x)`); no call-site inference in this effort |
| Dispatch | static only, through the monomorphizer |
| Receiver forms | `self`, `&self`, `&mut self` (existing method-receiver logic) |
| `Self` in signatures | supported (= `Var(0)`) |
| Coherence | reject two impls of the same interface for the same concrete self-type; no orphan/overlap analysis yet |

## Design (reuse, don't add machinery)

Four conventions make interfaces fit the current pipeline without touching the
engine/driver/flow subsystems:

1. **`Self = Var(0)`.** Interface method signatures lower with
   `TypeContext::new(&[Self, ...params])`, so `&self` → `&Var(0)`, a `Self`
   return type → `Var(0)`. Existing `substitute_terms`
   (`crates/hir/src/generics.rs`) substitutes these like any other generic
   parameter. This is the *only* new type-lowering convention; no new term
   kinds.

2. **Bounds as declaration metadata.** A `where T: Interface` clause is stored
   as `bounds: Vec<(u8 param_index, TermId interface_ty)>` on
   `HirFunctionDeclaration` (and on object methods), not as term nodes. The
   generic parameter itself is the existing `Var(i)`.

3. **Interface signatures are real function declarations.** Each interface
   method becomes a `HirFunctionDeclaration` with an empty body that is never
   enqueued. This gives the deferred call a `DeclarationId` target and reuses
   rendering, name lookup, and generic-argument handling wholesale. A caller of
   `x.stringify()` inside a bounded `T` body lowers to an `Ordinary`
   `HirExpressionKind::FunctionCall` whose target is that signature decl,
   `generics = [receiver_ty]`, receiver prepended via the existing reference
   logic in `build_field_access_impl` (`crates/hir/src/builders/expression/field_access.rs`).

4. **Impl store is one registry.** A new `Implementations` table (shaped like
   `MethodTable`, `crates/hir/src/context/types/methods.rs`) maps
   `(interface, self_template) → method decl ids`, plus a small
   `try_solve(template, concrete)` structural matcher over the term tree
   (recursion alongside `substitute_terms`) to solve generic impls
   (`impl<T> I for Wrapper<T>` vs `Wrapper<int>` → `T := int`). This is the
   entire "satisfies" machinery the feature needs; no engine, no proof search,
   no tabling.

### Resolution timing

The deferred interface call survives HIR building unresolved and is rewritten
by the monomorphizer:

- self type still a `Var` / contains null slots → keep deferred (same principle
  as today's nested generic calls);
- self type concrete → `try_solve` the impl template → `specialize` the impl
  method (existing cache/`in_progress`/cycle detection,
  `crates/monomorphizer/src/lib.rs`) → rewrite to a plain concrete
  `FunctionCall`;
- no impl for the concrete type → bounds-discharge error (new `HIRError` kind:
  e.g. `NoImplementationForInterface { interface, ty }`).

Concrete-type calls in non-generic code route through the same deferred
mechanism (like today's generic calls), so there is exactly one dispatch path.

## Required vs. required-not (from the extensible-core series)

Of the nine phases in `docs/contributing/extensible-core/08-implementation-order.md`,
only Phase 0 (the behavior matrix, done) plus a *slice* of Phase 4's
static-dispatch content are required. The rest is machinery other features
need, not interfaces:

| Phase | Required? | Why / slice |
|---|---|---|
| 0 Behavior matrix | yes (done) | the gate each step keeps green |
| 1 Terms | no new machinery | term tree already exists; interfaces add no term kinds (see design) |
| 2 Engine (solvers, queries) | no | static dispatch over concrete types is a table lookup |
| 3 Flow (CFG, ownership port) | no | unrelated |
| 4 Traits extension | partial | only: bounded generics, `impl I for T`, static dispatch via `specialize` |
| 5 Effects / 6 Typestate | no | — |
| 7 Driver (worklist, reification) | no | monomorphizer already discovers + specializes recursively |
| 8 Comptime / 9 Dynamic handlers | no | — |

## Implementation steps

Each step is a landable unit; `STD_PATH=./lib/std cargo test` must stay green
between steps.

1. **Lexer** — `crates/lexer/src/tokens.rs`: add `interface`, `impl`, `for`,
   `where` tokens. Confirm no identifier collisions across `examples/`, `lib/`,
   `tests/`.
2. **Parser/AST** — `interface Name<params> { func sigs }` → `InterfaceDeclaration`
   (no bodies/`=` in v1); `impl<params> Name for Type { bodies }` →
   `ImplDeclaration` (reuse `parse_method`, `crates/parser/src/objects.rs`);
   `where T: Interface` on funcs/object methods (extend `FuncDeclaration` and
   `Method` in `crates/parser/src/ast/functions.rs`). Wire into
   `parse_declaration` (`crates/parser/src/declarations.rs`) and `Program`.
3. **module_loader** — index `interface()` / `impl()` pools per entry;
   `find_interface` / `find_impl` honoring imports, mirroring `find_type`
   (`crates/module_loader/src/modules.rs`).
4. **HIR model** — `HirInterfaceDeclaration`, `HirImplDeclaration`, the
   `bounds` field on `HirFunctionDeclaration`, and the `Implementations`
   registry in `crates/hir/src/context/types/`.
5. **HIR builder** — hoist interface/impl declarations in `generate_hir`
   (`crates/hir/src/builders/mod.rs`); enqueue impl method bodies through
   `PendantFunction` with `self_type: Some(self_template)` (pattern:
   `resolve_method`, `crates/hir/src/builders/structs.rs`); lower `x.method()`
   on a bounded `Var` into the deferred call in `build_field_access_impl`
   (`crates/hir/src/builders/expression/field_access.rs`). All concrete-type
   paths are unchanged.
6. **Monomorphizer** — resolve deferred calls in `build_expression`
   (`crates/monomorphizer/src/lib.rs`): concrete self → solve impl → specialize
   method → rewrite call; still generic → keep deferred; no impl → error.
   Generic impl-method templates are neutralized as dead code like other
   generic functions. **Codegen: no changes** (everything is a plain function
   call afterward).
7. **Coherence (minimal)** — reject duplicate `impl I for T` for the same
   concrete self type at first dispatch.
8. **Gates** — `examples/interfaces/` corpus with `// xfail:` / `// xpass:`
   markers + `tests/interfaces.rs` using the `tests/generics.rs` harness
   pattern. New BM-7xx rows in `docs/contributing/extensible-core/09-behavior-matrix.md`
   and a "Phase 4R (interfaces, reduced)" entry in its §5 hook table.

### Gate scenarios

- Positive: concrete interface call; bounded generic `func describe<T>(x: T)
  where T: Stringifiable`; generic impl; two interfaces on one type; `Self`
  return type; `&mut self`; `impl` in a different file via import.
- Negative: interface method call on an **unbound** `T` (no `where`);
  interface method call when the concrete type has **no impl**; duplicate
  `impl I for T`.
- `xpass` (eventual, documented gaps): associated-type-projection usage,
  supertraits, `dyn`/trait objects — must compile or be marked.

## Relevant files

| What | Where |
|------|-------|
| Behavior matrix (the gate) | `docs/contributing/extensible-core/09-behavior-matrix.md` |
| Tokens | `crates/lexer/src/tokens.rs` |
| Declaration parsing / Program | `crates/parser/src/declarations.rs`, `crates/parser/src/ast/` |
| Method-body parsing (reused by `impl`) | `crates/parser/src/objects.rs` |
| Func signature / `where` hook | `crates/parser/src/functions.rs`, `crates/parser/src/ast/functions.rs` |
| Import-aware lookup (mirrors) | `crates/module_loader/src/modules.rs` |
| Declaration model | `crates/hir/src/model/declarations.rs` |
| Type/method tables (registry pattern) | `crates/hir/src/context/types/{mod,methods}.rs` |
| Generic substitution (`Self = Var(0)`) | `crates/hir/src/generics.rs` |
| Deferred-call lowering | `crates/hir/src/builders/expression/field_access.rs`, `.../calls.rs` |
| Impl body hoisting (self_type pattern) | `crates/hir/src/builders/{mod,function,structs}.rs` |
| Dispatch / specialization | `crates/monomorphizer/src/lib.rs` |
| HIR error kinds | `crates/hir/src/error.rs`, `src/compilation_context/errors/hir.rs` |

## Deferred (incremental later, on the same seams)

- Associated types / projections (needs the engine's `normalize` slot or a
  projection term — pick then)
- `dyn Interface` + vtable synthesis (needs driver reification)
- Supertraits, default method bodies (manifest sugar, no core change)
- Orphan/overlap coherence for generic impls
- Engine solvers, driver worklist, terms kinds/stages — only if/when the
  extensible-core series resumes

## Common pitfalls

- **Don't add a new expression kind** for the deferred call unless `Self` needs
  to carry more than the existing `generics` slot; start with a `FunctionCall`
  to an interface-signature decl.
- **`try_solve` must substitute, not just pattern-match**: a generic impl's self
  template may nest (`Option<Vec<T>>`); reuse the `substitute_terms`
  recursion shape.
- **Impl methods are generic only in impl params.** Keep `Self` out of the
  impl method's own generic list or the arity checks in
  `Monomorphizer::specialize` will miscount.
- **Hoist interfaces/impls from every entry**, not just the entry that reaches
  `main` — a call resolves to an impl the entry may not reference at first.
- **Coherence check must key on the *solved* self type**, not the template:
  `impl I for Person` and `impl I for AnotherShapeByName` are the same type.