# Comptime as an Extension

> Series: *extensible-core*. Assumes [`00-overview.md`](00-overview.md). The most
> *structural* of the six features: it does not just add rules or a lattice — it
> changes the shape of the compiler's day (linear pipeline → worklist fixpoint).

---

## 1. The Feature

Comptime is **code that runs while compiling**, written in the *same language*
the user writes, on the *same terms* the compiler already has — no separate
macro language, no template metaprogramming dialect, no external build step.

```slynx
/* type as a value: a comptime parameter that IS a type */
func make_arr(comptime n: int) : [n]u8 { ... }

/* comptime block: compute a type, then use it */
comptime {
    let t = if (target_is_little_endian()) { u16 } else { u8 };  // a "type" value
    // ... use `t` to specialize a later call ...
}
```

Two promises, both demanding:

1. **Same language.** The comptime evaluator is the compiler *itself*, running
   on the compiler's own representation. There is no "which subset of the
   language can I use in a macro" — the answer is "everything, minus whatever
   the runtime actually needs (which you can't touch at comptime anyway)."
2. **Types are values.** A type (the *kind* `Type` from the HKT doc) is a
   first-class value that comptime code can compute, branch on, and hand to a
   generic. "Generics are just comptime over types" falls out: a generic
   function body run at comptime with concrete type arguments is *mono-
   morphization*.

---

## 2. The Mechanism in Real Systems

### 2.1 Zig: same-language, phase-forked interpretation

Zig's comptime is the closest cousin of what Slynx should build, and its
internal mechanism is the valuable part:

- **Phase forking.** The compiler tracks, per value, whether it is
  `comptime`-known or not (Zig's *Sema* maintains a compile-time flag in the
  AST/type walk; lazily, only where it must). This is the *staging flag* idea
  from the overview (3.2) — the core will carry the exact same two-valued
  marker.
- **Types as values.** `comptime T: type` binds a *type* to a named value; a
  generic `fn foo(T: type, x: T)` is then *just* a function that happens to
  take a type. Monomorphization in Zig is "call a function with a `type`
  argument at comptime", and the *compiler's cache is a straight runtime-value
  graph of the interpreter*, not a separate pass.
- **Lazy, driver-driven evaluation.** Zig does not evaluate every `comptime`
  block eagerly; it schedules work on demand, *triggered by usage*. The reader
  who has internalized the overview (section 3.5) recognizes this as a
  **worklist + fixpoint**, not an eager pre-pass. *This* is the structural
  consequence of comptime — a compiler that has comptime is a compiler whose
  top-level loop is *"while there is work, resolve the next demand"*.
- **Reification.** The Zig semantic action *reifies* a comptime-known value
  into IR at the point where runtime code needs it (a comptime-computed length
  becomes a constant in an array type literal, etc.). Slynx's overview names
  this operation `REIFY` (3.5) and makes it a driver hook the comptime
  extension fills.

### 2.2 Multi-stage (MetaOCaml): staging a *type* tool

MetaOCaml's two-level type system assigns each expression a *stage*
(now / later): a value has type `'a code` if it is a *future* computable, and
`build` composes staged pieces while `run` forces them. Its mechanism is
**type-annotated staging** — the compiler refuses to mix stages statically.
Slynx's staging *flag* (CompileTime/Runtime) is the same idea, simplified: the
comptime extension owns the *type-level* law ("a runtime-only value may not be
used where a comptime value is demanded"); the core just carries the flag.

### 2.3 Template Haskell / Rust const generics: what NOT to copy

Template Haskell reifies *AST snippets* and evaluates them via a full compiler
pass; its "Turbo fish" problem is having a separate syntax and separate name
resolution for macros. Rust's const generics (`const N: usize`) plus `const fn`
evaluation is a *narrowed* comptime (a restricted interpreter over MIR) that
works because Rust does not need comptime to *compute types*. Slynx *does*
(types-as-values is the headline), so it needs the broader Zig-shaped model,
not Rust's narrowing.

---

## 3. What Slynx Already Has

Honest inventory: *nothing* is comptime today — except three seeds that are,
as it happens, exactly the seeds comptime needs:

| Seed | File | Comptime meaning |
|------|------|------------------|
| Monomorphizer **cache** + `in_progress` + `dead_code` | `crates/monomorphizer/src/lib.rs` | The *first* comptime answer-store: "specialize(F, Args) has (or has not) been computed." |
| The staged terms *flag* was specified in the overview but not yet implemented | (design) overview 3.2 | comptime's runtime/comptime marker. |
| Worklist/`in_progress` frontier | `crates/monomorphizer/src/lib.rs` | The seed of the driver fixpoint (overview 3.5.1). |
| An infamous comment in the HIR array builder | `crates/hir/src/builders/mod.rs` — array-length construction noted "idealized to be replaced by comptime in the future" | The language already *knows* the right tool is comptime. |
| Generic parameter substrate | `crates/hir/src/generics.rs` | The `substitute_types` step is comptime substitution over types — reusable for *value* substitution too. |

The gap is not a missing pass; it is **a driver that can pause and grow.**
Today `CompilationStages::build_stages()` (`src/compilation_context/mod.rs`)
is linear: load → build HIR → monomorphize → codegen. Comptime cannot exist
in a linear compile because "evaluate a comptime block" *produces more work*
(a new type to specialize, a new `impl` to synthesize for traits, a new array
length to fold). The linear loop must become *the fixpoint loop*.

---

## 4. The Extension Design

```text
extension "comptime":
  order: 15           # after terms (staging flag) and the driver worklist; before
                      # traits specialize (its "synthesized impls need a better driver")

  syntax:  comptime marker ::= `comptime ( expr | block | param )`

  declaration_kinds:
    comptime_var  ::= data(name, kind: Type | value-kind, value)  # a comptime binding

  type_terms:
    staged_value  ::= arity: 1, kind: Type, stage: CompileTime   # "a known value"
    (the core's terms already carry the stage flag; this extension *names* it)

  engine (queries the evaluator answers):
    eval(expr, env) -> CompileTimeValue          # the interpreter, memoized
    kind_of(Term) -> Kind                        # reuse the HKT ext's kind service
    pure_fn(FnDecl) -> bool                      # may this fn run at comptime?
    specialize(Fn, ArgList) -> Sig               # reroute to the monomorphizer's cache
       (the monomorphizer becomes a *query consumer*: specialize() is these days
        the answer a comptime call needs, and the one a trait impl needs too)

  driver hooks:
    work_item scheduled when:  a comptime block references a type/generic call
                               a stage-CompileTime value is demanded at Runtime
    REIFY:                     a value from eval() becomes an IR literal (array
                               length, const, specialized fn ref)
    stage-check law:           Runtime-sourced values may not flow into a
                               CompileTime slot  (MetaOCaml's law, enforced here)
```

### 4.1 The interpreter

The comptime evaluator is `eval(expr, env)`: a tree-walking interpreter over
**HIR** (`PoolId<HirExpression>`, `HirStatement`, the place machinery of
`crates/hir/src/ownership/place.rs`) that reuses the *existing* expression
machinery rather than inventing a new language. It is "the compiler's own
evaluator" in the truest sense: the same builders that *type-check* an
expression can *evaluate* it, because the HIR already stores resolved
operations (`Binary`, `FunctionCall`, component constructions) with typed
children. Zig's interpreter similarly walks its own AST with a memo; there is
no separate comptime front-end.

- **Termination law.** Comptime is `eval` with a step budget and a recursive
  `comptime` call-guard (Zig errors on unbounded comptime recursion); the
  driver treats a live-budget exhaustion as a diagnostic ("comptime didn't
  converge"), never as a hang.
- **Purity law.** A comptime-callable function must be `pure_fn`-provable —
  otherwise the engine cannot cache its answers, and caching is half the point
  (the salsa-style memoization argument of the overview 3.3).
- **Types are just values.** `eval` over a `Type`-valued result returns an
  arena *type term id* (the terms layer, overview 3.2). Handing that id to a
  generic call is exactly "using a type as a value"; the monomorphizer's
  `specialize` query accepts either a source type or a comptime-computed one.

### 4.2 The driver worklist, made concrete

The pipeline becomes:

```text
                 ┌──────────────────────────────────────────┐
                 │              DRIVER LOOP                 │
                 │  pop work ▸ compile one unit ▸ queries   │
                 │       ▲              │        ▲          │
                 │       │              ▼        │          │
                 │  work queue ◀──(specialize <>)─┘          │
                 │       │              ▲                    │
                 │       │   comptime eval() demands         │
                 │       └──────────────┘                    │
                 └──────────────────────────────────────────┘
                    exit when the queue empties (a fixpoint)
```

The monomorphizer stops being "a pass that runs after HIR build" and becomes
"the thing that answers `specialize(F, Args)` queries, scheduling work when the
answer isn't cached." Trait impls (traits doc section 4.2) and effects
evidence (effects doc section 4.2) are *children of the same loop*: both
schedule, both reify, both ask the engine. **One compiler day**, not six
days that all run for the whole program.

---

## 5. The Concrete Example (`make_arr`, then a trait-generation one-liner)

```slynx
func make_arr(comptime n: int) : [n]u8 {
    let mut buf: [n]u8 = zeroes;   // the array type is comptime-constructed
    return buf;
}

func entry() {
    let a = make_arr(4);          //  eval: n=4 → [4]u8 → specialize(make_arr, [4])
    let b = make_arr(16);         //  eval: n=16 → [16]u8 → specialize(make_arr, [16])
}
```

Mechanically: `make_arr`'s body lower has a *staged* `n`; `buf`'s type is a
`[n]` term with `n` still CompileTime; the call `make_arr(4)` demands a
Runtime `[4]u8`, the driver invokes `eval(4, env)` → pushes `specialize(make_arr,
[4])` → the monomorphizer instantiates; `REIFY` turns the comptime `4` into an
IR constant in the array type. The entire feature is "eval + specialize +
reify."

### The trait/impl synergy

```slynx
/* comptime 突然 gives traits a second, free superpower */
comptime {
    let t = if (std.comptime_target_has_sse()) { Simd8<f32> } else { AlignedVec<f32> };
    impl_vec_dist(t);                    // calls the generic `impl_vec_dist<T>(x: Vec<T>)`
}
```

`impl_vec_dist`'s bounds (`where T: Dist`) are discharged by the *same*
worklist that mono'd `make_arr`: `satisfies(Simd8<f32>, Dist)` is a query,
answered by the trait impl store, and — the comptime twist — a comptime block
can *add* an impl (via the traits ext's reification) and *search again*: the
query cache is invalidated by the driver, per overview 3.3. That loop ("comptime
added a fact; re-resolve the affected goal") is the salsa contract doing what
Zig does by virtue of being an interpreter over a monomorphizing engine.

---

## 6. Alternatives Considered and Refused

1. **A separate macro language (Racket-style syntax objects / TH).** Refused:
   it duplicates the user language's grammar, its name resolution, its type
   checking, and its docs. Slynx *has* a tiny syntax already
   (`docs/language-surface.md`); forcing a second one (macro DSL) makes
   comptime a word-for-word copy of the language with new grammar rules —
   the exact inefficiency the "same language" promise exists to avoid.
2. **Template Haskell-style typed reification.** Refused for v1: TH's
   *splice/quote* syntax is powerful but its reification typing is a second
   staging type system. MetaOCaml's annotation was honestly more principled;
   but Slynx's needs are served by the *flag* + *law* (a runtime value can't
   flow into a comptime slot) — a weaker but far cheaper guarantee.
3. **External build step / preprocessing.** Refused: it splits compilation,
   breaks incremental compilation and the language server story, and cannot
   *type-check around* its own output (the trait/impl synergy above becomes
   impossible if comptime is a separate program).
4. **Zig's exact implementation (a full evaluator inside Sema).** Not refused,
   but scoped: Zig's interpreter runs over its own AST/IR with a careful
   upgrade model for `comptime` annotation; Slynx's stays a *thin* tree-walker
   over HIR because our feature set is smaller — and the loop that drives it
   (worklist fixpoint) is bought here as a *core* change, not an extension
   patch (impl order doc, [`08`](08-implementation-order.md) section 6, is
   explicit that the driver refactor is deliberately scheduled, not accidental).

---

## 7. Compatibility Fallout

- **The linear pipeline dies.** `CompilationStages::build_stages()` becomes the
  driver loop. This is the *largest single architectural disruption* in the
  series and the reason impl-order schedules it *last* (after terms, engine,
  flow, and the ownership/typestate ports have shaken out the loop's shape on
  *known* work).
- **Queries must become pure + memoized + invalidatable.** The engine/salsa
  contract (overview 3.3) is no longer a nicety — comptime depends on
  re-resolution after comptime-added facts. Code that answers `method`,
  `satisfies`, `specialize` *must* go through the query cache; short-circuits
  that read `MethodTable` directly (`crates/hir/src/context/types/methods.rs`)
  are comptime hazards to be extirpated.
- **Types become staged.** The terms layer now carries CompileTime/Runtime; the
  monomorphizer's *cache key* (a term list) and the codegen's `TypeLowerer`
  (`crates/codegen/src/lowerers/mod.rs`) must tolerate and skip comptime-only
  terms (they never reach IR; `REIFY` erases them).
- **Existing monomorphizer tests** (`tests/monomorphizer.rs`) survive
  unchanged (the specialize query answers identically); only its *callers*
  change.

---

## 8. Cross-References

- The **staging flag** and **REIFY** hook are core (overview 3.2, 3.5).
- **Kinds as structural plumbing** (HKT doc section 3) is what "type as value"
  means: `comptime T: type` binds a *kind-typed* value; the comptime evaluator
  and `kind_of` are siblings.
- **Queries revisit after comptime-added facts** is the salsa contract
  (overview 3.3); the effects doc's *dynamic-handler* and the traits doc's
  *synthesized impl* are the same rev-@comptime pattern.
- **The tensions doc** ([`07`](07-tensions.md) section 6) argues that the
  current hardcoded `assert_no_generic_non_functions` limits (aliases,
  stylesheets) are *symptoms of a linear compiler* and dissolve in the fixpoint
  driver.

---

Next: [`06-move-semantics.md`](06-move-semantics.md) — repackaging the one
feature that already exists, from a hardcoded pass into the flow extension that
typestate stands on.