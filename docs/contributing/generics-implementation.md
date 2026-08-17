# Generics Implementation Plan

> Status: **in progress** — the parser and AST steps (checklist 1–3) plus the
> parser tests of step 12 are implemented. HIR, type-checker, and
> monomorphizer work remains.
> This document is the result of a full-codebase analysis. It describes what
> exists today and an ordered, dependency-driven plan to implement **basic
> unconstrained generics for functions** with type checking and monomorphization.

## 0. Scope

### In scope (the entire milestone)

- Generic functions: `func identity<T>(x: T): T`
- Unconstrained type parameters only — `<T>` with no bounds, no defaults.
- Explicit use-site application: `identity<i32>(x)`
- **Type checking** of generic function bodies and of calls to generic
  functions, performed in the HIR builder.
- **Monomorphization**: one specialized function per unique type-argument list,
  memoized, with concrete types substituted everywhere.

### Explicitly out of scope (do NOT implement)

- Generic **objects** (`object Nullable<T>`) and generic **components** — and
  therefore `HirType::Reference` construction, and any
  `TypesContext`/`SymbolRegistry` object-generic work. The `Reference` variant
  stays dead code.
- **Concepts / trait bounds** (`T: Concept`, `concept` keyword).
- **Const generics** (`<const N: usize>`, symbolic array sizes, `const` keyword).
- **`extend<T> []T { ... }`**.
- **Type inference** at call sites (`identity(x)` without explicit `<...>` args).
  Requires collecting unification bindings; noted as a cheap follow-up since
  `unify_types` already accepts free params (§6.2).
- Generic stylesheets.

Consequences of the reduced scope: no lexer changes are needed (no new
keywords), no `Program` changes are needed (no new declaration kinds), and
codegen needs no changes (specializations reuse the existing declaration
hoisting).

---

## 1. Ground truth about the pipeline

The documented pipeline (`docs/contributing/project-organization.md`) lists a
`crates/checker` stage between HIR and the monomorphizer. **That crate does not
exist.** Type checking is performed inline while the HIR is built:

```
source → lexer → parser → AST → [HIR + type check] → monomorphizer → codegen → SIR
```

- Type resolution and checking live inside the HIR builders:
  `crates/hir/src/builders/mod.rs` (`HirNode::find_type`,
  `get_signature_of_function`) and `crates/hir/src/builders/expression.rs`
  (`ExpressionBuilder::build_expression`, `unify_types`, `lookup_function`).
- The monomorphizer is a stub (`crates/monomorphizer/src/lib.rs`), called from
  `src/compilation_context/mod.rs` (`SlynxContext::build_hir` →
  `SlynxContext::monomorphize`).
- There is **no JS backend in this repository**; the pipeline ends at a textual
  `.sir` IR.

All "type checker" work below happens in the HIR builder.

---

## 2. Parser

### Current state

- `parse_type` (`crates/parser/src/types.rs:94`) parses use-site generic
  instantiation `identity<i32>` into
  `Type::Plain(GenericIdentifier { generic, identifier })`
  (`crates/parser/src/ast/types.rs:24`). `Parser::is_generic` (`types.rs:121`)
  is the lookahead that tells generics apart from the `<` comparison operator;
  `Parser::is_generic_application` (`types.rs:246`) is the bounded lookahead
  used at call sites, stopping at the closing `>`.
- `parse_func` (`crates/parser/src/functions.rs:26`) parses the declaration name
  with `parse_type()`, then `split_type_params()` (`types.rs:87`) splits any
  `<T>` off the name into a declaration-site parameter list stored in
  `FuncDeclaration.type_params`, and `push_type_params()` (`types.rs:108`)
  interns each param into the function scope so `T` resolves to
  `Type::Generic(i)` inside the body.
- `parse_identifier_exprs` (`crates/parser/src/expr.rs:181`) routes `Name<...>`
  by the token after the closing `>`: `(` → generic **function call**
  (`parse_funcall_args`), `{` → **component** expression (as before).
- `ASTExpression::FunctionCall { name: Spanned<DedupPoolId<Type>>, args }`
  (`crates/parser/src/ast/expression.rs:60`) carries the callee as a `Type`, so
  a use-site `identity<i32>` in a call lives in the name's `GenericIdentifier`.

### Implemented

1. `parse_func` (`functions.rs`): parses the name with `parse_type()`, then
   `split_type_params()` pulls a declaration-site `<T>` off the name into
   `FuncDeclaration.type_params` (a bare identifier list — **no bounds, no
   const**), and `push_type_params()` interns each param into the function
   scope so `T` resolves to `Type::Generic(i)`.
2. `parse_identifier_exprs` (`expr.rs:186`): when `is_generic_application()`
   is true, look at the token after the closing `>`:
   - `(` → `parse_funcall()`, which parses the name via `parse_type()` and keeps
     `identity<i32>` in the call's name `Type`;
   - `{` → component expression, as before.
3. Generic calls **in method/postfix position** (e.g. `obj.method<i32>(x)`) are
   out of scope for this milestone; `parse_dot_postfix` stays as is.

---

## 3. AST

### Current state

- `FuncDeclaration` (`crates/parser/src/ast/declarations.rs:50`) has
  `name: Spanned<DedupPoolId<Type>>` plus the new
  `type_params: Vec<Spanned<DedupPoolId<Type>>>`.
- Use-site generic arguments already have a home: the name `Type` is a
  `Type::Plain(GenericIdentifier)` — true both for type positions and for
  `FunctionCall.name`.
- `Program`/`SourceNode` plumbing is untouched: no new declaration kind is
  introduced by this milestone.

### Implemented

1. `type_params: Vec<Spanned<DedupPoolId<Type>>>` is on `FuncDeclaration`
   (`ast/declarations.rs:50`), filled by `split_type_params` in `parse_func`
   (§2). — *low*
2. No new AST nodes. `GenericParam`/`GenericBound` structs are **not** needed —
   the split identifier list fully represents an unconstrained parameter
   list.

### Estimated complexity

**Low.** One field on one struct plus one parser constructor site.

---

## 4. HIR

### Current state

- `HirType` (`crates/hir/src/model/types.rs`) is a flat enum over concrete
  types. **There is no way to represent an unresolved generic parameter `T`.**
  Every type must resolve to a concrete `DedupPoolId<HirType>`.
- `HirFunctionDeclaration` (`crates/hir/src/model/declarations.rs:67`) has
  `name, args, statements, ty, visibility, external, attributes` — no
  type-parameter list.
- Function bodies are built through a work channel: `HirQueueBuilder`
  (`crates/hir/src/builders/mod.rs:66`) sends a
  `PendantBody { func_id, body, argument_names }` (`mod.rs:55`) and
  `process()` replays the **AST** through `ExpressionBuilder` per body.
  `HirFunctionBuilder::create_argument` reads each argument's type from the
  hoisted signature.
- `HirExpressionKind::FunctionCall { name: DeclarationId<HirFunctionDeclaration>, args }`
  (`crates/hir/src/model/expression.rs`) — call sites point at a **specific
  declaration id**. Good: a specialization just needs its own id.
- `DedupPoolId<HirType>` dedupes into a **single global pool**, so a bare
  `HirType::GenericParam(u8)` would collide across functions (`T` in function A
  vs `T` in function B). The parameter must be tagged with its owner.

### What must change (in dependency order)

1. Add a parameter variant to `HirType` that is safe in the global pool:

   ```rust
   // model/types.rs
   HirType::GenericParam {
       /// the generic function this parameter belongs to
       owner: DeclarationId<HirFunctionDeclaration>,
       /// index into the function's type-parameter list
       index: u8,
   }
   ```

   Hashable, dedupe-safe, and lets the specialization machinery know exactly
   which list to substitute against. — *low*
2. Add `type_params: Vec<SymbolPointer>` to `HirFunctionDeclaration`
   (`model/declarations.rs:67`), populated from the AST in
   `create_func`/`get_signature_of_function`. — *low*
3. Give `ExpressionBuilder` a type-parameter environment:
   `type_params: HashMap<SymbolPointer, DedupPoolId<HirType>>`, populated from
   the function's `type_params` at body-build time (each `T` → its
   `HirType::GenericParam` id). Thread it through `PendantBody` (add a field)
   and into `HirFunctionBuilder` so every nested
   `ExpressionBuilder::build_expression` sees it. — *medium*
4. `find_type` (`builders/mod.rs:92`) must resolve a bare name that matches a
   *declared type parameter* to its `GenericParam` id **before** falling back to
   module lookup. Because `find_type` lives on `HirNode` (which has no per-body
   state), it must receive the environment — as a parameter or via a wrapper
   (`find_type_in_env`), updating every call site
   (`get_signature_of_function`, `build_statement_data`, `build_expression`). — *high*

Note: `HirType::Reference { rf, generics }` (and the `dereference()` infinite
loop bug at `crates/hir/src/helpers/views/types.rs:102`) are **not** touched.
No `Reference` is ever constructed in this milestone (objects are out of scope),
so that path stays dead code.

### Estimated complexity

**High** for the `find_type` rework and env threading; **low** for the new
variant and the field addition.

---

## 5. Type checking (the HIR builder)

### Current state

- `HirNode::find_type` (`builders/mod.rs:92`) resolves an AST `Type` to
  `(FileId, DedupPoolId<HirType>)` via `Modules::find_type_inside_module`
  (`crates/module_loader/src/modules.rs:171`), matching builtins/structs/etc.
  **by name only.** Today, `T` in `func identity<T>(x: T): T` fails here with
  `HIRError::type_unrecognized` because "T" is looked up in the module and not
  found.
- `ExpressionBuilder::unify_types` (`builders/expression.rs:108`) compares two
  `DedupPoolId<HirType>` and raises `HIRError::unexpected_type` on mismatch.
- `ExpressionBuilder::lookup_function` (`builders/expression.rs:160`) resolves a
  call's name `Type` to a `DeclarationId` via `SymbolRegistry` + file pools,
  **dropping** the `GenericIdentifier.generic` args (`queue.type_name`).
- `HirNode::get_signature_of_function` (`builders/mod.rs:169`) computes each
  function's `FunctionType { args, ret }` once, hoisted before bodies are built.

### What must change

1. **Name → parameter resolution.** With §4.4 in place, a bare name matching a
   declared type parameter resolves to its `GenericParam` id. This single change
   makes `func identity<T>(x: T): T` type-check. — *low*
2. **`unify_types` accepts `GenericParam` freely.** If either side is a
   `GenericParam`, unification succeeds (no constraint is collected). This is
   what lets a body mentioning `T` in multiple positions agree, and what lets a
   call with explicit concrete args match the generic signature. — *low*
3. **Generic call checking + specialization request (the core mechanism).** In
   `lookup_function` / the `FunctionCall` arm of `build_expression`:
   - extract the call's explicit generic args from the name's
     `GenericIdentifier.generic`; if none are present and the callee is generic,
     error (inference is out of scope);
   - resolve each arg to a concrete `DedupPoolId<HirType>`;
   - look up the callee's `FunctionType` (contains `GenericParam` markers) and
     **substitute** (§6.2) to get the specialized signature;
   - **request the specialization** from the registry (§6.1) to obtain the
     specialized `DeclarationId`;
   - unify the call args against the substituted arg types. — *high*
4. **Nested generics fall out for free.** A generic body calling another generic
   with a parameter (`func foo<T>(x: T): T { return identity<T>(x); }`) is built
   with the **concrete** environment of its own specialization, so the inner
   call's explicit arg resolves to a concrete type at build time. No deferred
   resolution machinery is needed.

### Estimated complexity

**Medium.** Items 1–2 are small; item 3 is the real work but is localized to
`lookup_function` and the `FunctionCall` arm.

---

## 6. Monomorphizer

### Current state

`crates/monomorphizer/src/lib.rs` — 37 lines, a stub. `resolve()` does nothing.
Runs after `SlynxHir::new` inside `SlynxContext::build_hir`
(`src/compilation_context/mod.rs:341`), before codegen.

### Design: eager specialization during HIR generation

Bodies are built by replaying the **AST** through `ExpressionBuilder` per
`PendantBody`, so a specialization is just another `PendantBody` enqueued under
a concrete type-parameter environment and a different target `DeclarationId`.
This is **approach (A)** from the full proposal and it is the one this plan
uses: specialization happens at call sites, driven by a registry shared with the
monomorphizer crate.

### What must change

1. **Specialization registry.** A memo table
   `HashMap<(DeclarationId<HirFunctionDeclaration>, Vec<DedupPoolId<HirType>>),
   DeclarationId<HirFunctionDeclaration>>` plus `specialize()`:
   - if the key exists, return the cached id (two call sites with the same args
     → one specialization);
   - otherwise build a new `HirFunctionDeclaration` whose signature is the
     template's signature with `GenericParam`s **substituted** by the concrete
     args, give it a **mangled name** (e.g. `identity__i32`; format is a
     decision), push it into the same per-file `DeclarationsPool`
     (`HirFile.declarations`) so `find_function_with_name` never collides with
     the template, and enqueue its `PendantBody` with the concrete env.
   
   The registry must be reachable both while `SlynxHir::new` builds bodies and
   from `monomorphizer::resolve`; natural home is the `SlynxHir` context (next
   to `TypesContext`), with the monomorphizer crate owning the API. — *medium*
2. **Type substitution.** `substitute(ty, &[(index) -> TypeId])` that recurses
   into `Function`, `Tuple`, `Array`/`Vector`, and replaces `GenericParam` leaves.
   Reused by the checker (§5.3) for specialized signatures and by the registry
   when cloning the template. — *medium*
3. **`Monomorphizer::resolve`.** After `SlynxHir::new`, it (re)processes any
   specialization work that was enqueued but not yet drained (same drain loop
   `HirQueueBuilder::process` already runs), reports unsatisfiable requests
   (e.g. a generic called with the wrong arity of type args), and verifies the
   registry is consistent. — *medium*

Alternative (not recommended, documented for completeness): a pure
clone-based pass that keeps bodies typed with `GenericParam` and clones +
substitutes every statement/expression, re-owning `VariableId`s and re-mapping
nested call sites. Requires a deep HIR cloner — significantly more machinery —
and buys nothing here because bodies are already rebuilt per specialization.

### Estimated complexity

**Medium.** Memoization and naming are low; substitution and the registry↔build
sharing are the non-trivial parts.

---

## 7. IR and codegen

### Current state

- `IRType` (`crates/ir/src/types/irtype.rs`) is fully concrete — no generic
  types, and none are needed: by codegen time every generic has been replaced by
  a concrete specialization.
- Codegen is keyed off HIR ids: `hoist_declarations`
  (`crates/codegen/src/lib.rs:101`) creates one IR function per HIR declaration
  and fills `self.functions: HashMap<DeclarationId, IRPointer<Function,1>>`;
  `lower_function_call` (`crates/codegen/src/expressions.rs:61`) looks up the
  **specific** `DeclarationId` → IR function.
- No JS backend exists in the repo (output is `.sir`).

### What must change

- **Nothing.** Specialized `HirFunctionDeclaration`s live in the same per-file
  pools as templates, so `hoist_declarations` picks them up automatically and
  creates distinct IR functions under their mangled names; every call site
  already points at a concrete `DeclarationId`. No `IRType`, `IRFunction`, or
  naming changes.

### Estimated complexity

**None.**

---

## 8. Ordered implementation checklist

Each step depends only on the steps above it. **M1** is the entire scope.

| # | Step | Depends on |
|---|------|-----------|
| 1 | AST: `type_params: Vec<Spanned<DedupPoolId<Type>>>` on `FuncDeclaration` (`ast/declarations.rs`) — **done** | — |
| 2 | Parser: `split_type_params()` in `parse_func` (`functions.rs`); declaration name becomes plain; `<T>` no longer conflated with use-site args — **done** | 1 |
| 3 | Parser: generic call path `Name<T...>(...)` in `parse_identifier_exprs` (`expr.rs:186`), using `is_generic_application` + `parse_funcall` — **done** | 1 |
| 4 | HIR: add `HirType::GenericParam { owner, index }` (`model/types.rs`) | 1 |
| 5 | HIR: `type_params` on `HirFunctionDeclaration` (`model/declarations.rs`), populated from AST | 1, 4 |
| 6 | HIR: `ExpressionBuilder.type_params` env + `PendantBody` field, threaded into `HirFunctionBuilder` (`builders/mod.rs`, `builders/function.rs`) | 5 |
| 7 | HIR: `find_type` resolves declared type params to `GenericParam` before module lookup (env-aware `find_type_in_env`, all call sites updated) | 4, 6 |
| 8 | Checker: `unify_types` accepts `GenericParam` freely; `func identity<T>(x: T): T` type-checks | 7 |
| 9 | Monomorphizer: specialization registry (memo table, mangled names, pool insertion, body enqueue) + `substitute()` type walker | 5, 8 |
| 10 | Checker: `lookup_function`/`FunctionCall` arm reads explicit generic args, substitutes, requests specialization, unifies args against substituted signature | 9 |
| 11 | Monomorphizer: `resolve()` drains pending specialization work and validates the registry | 9, 10 |
| 12 | Tests: parser tests for `func f<T>` and `f<i32>(x)` — **done**; HIR/type-checker tests for `identity<T>`, nested-generic calls; end-to-end `compile_source` tests through `.sir`; dedupe test (two call sites, one specialization) | 2–11 |

### Verification

- Run `cargo test --workspace` after each step.
- End-to-end harness lives in `tests/common/mod.rs`; existing type-checker /
  monomorphizer tests in `tests/type_checker.rs` and `tests/monomorphizer.rs`.

---

## 9. Open design decisions

- **Mangled specialization names** (§6.1) must be stable and unique for codegen
  and any future backend; format is a decision (e.g. `identity__i32` vs
  `identity$0`).
- **Registry home** (§6.1): on `SlynxHir` next to `TypesContext`, with the API
  in the monomorphizer crate.
- **Inference** at call sites (no explicit `<...>`) is a deliberate follow-up;
  `unify_types` already unifies free params, so it is a matter of collecting
  bindings at the call site and feeding them to the registry.

---

## 10. Key file map

| Concern | File |
|---|---|
| AST function declaration | `crates/parser/src/ast/declarations.rs` |
| AST generic identifier | `crates/parser/src/ast/types.rs` |
| Call expression node | `crates/parser/src/ast/expression.rs` |
| Func parsing | `crates/parser/src/functions.rs` |
| Type parsing + `is_generic` / `is_generic_application` | `crates/parser/src/types.rs` |
| Expr parsing (calls/components) | `crates/parser/src/expr.rs` |
| Module lookup (types by name) | `crates/module_loader/src/modules.rs` |
| `HirType` | `crates/hir/src/model/types.rs` |
| HIR function declaration | `crates/hir/src/model/declarations.rs` |
| `DeclarationId` / `VariableId` | `crates/hir/src/id.rs` |
| Type resolution / signature | `crates/hir/src/builders/mod.rs` |
| Body building / type checking | `crates/hir/src/builders/expression.rs` |
| Function body queueing | `crates/hir/src/builders/function.rs` |
| Monomorphizer (stub) | `crates/monomorphizer/src/lib.rs` |
| Pipeline wiring | `src/compilation_context/mod.rs` |
| Codegen declaration hoisting | `crates/codegen/src/lib.rs` |
| Call lowering | `crates/codegen/src/expressions.rs` |
| End-to-end test harness | `tests/common/mod.rs`, `tests/type_checker.rs`, `tests/monomorphizer.rs` |
