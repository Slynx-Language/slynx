# Implementation Order: Landings That Minimize Rework

> Series: *extensible-core*. Assumes [`00-overview.md`](00-overview.md) and the
> companion docs. This document answers the brief's final deliverable: a
> recommendation of the order in which to build the movable core so that
> **nothing built early has to be torn down by anything built later.** Every
> landing is gated by the existing test suite
> (`tests/`, `examples/`, `docs/`) plus the behavior contracts the feature docs
> pinned down.

---

## 1. The Invariant That Dictates the Order

Every phase must leave the compiler **strictly more capable and never less
correct** than the phase before it. The way to guarantee that is to follow one
dependency law:

> **Build the substrate first and twice:** once structurally (Terms), once
> semantically (Engine), once analytically (Flow). Every *feature* is a
> re-expression of an existing mechanism on that substrate — never a new
> mechanism.

That law produces this dependency graph, and the order below is a topological
sort of it that also minimizes *visible breakage windows*:

```
Phase 1  Terms (term tree, kinds, staging, unifier)
   │
   ├─────────────▶ Phase 2  Engine (rules, two solvers, query cache)
   │                        │
   │                        ├─────────────▶ Phase 4  Traits (bounds, impls, projections)
   │                        │                        │
   │                        │                        └──▶ (generic aliases/stylesheets)
   │                        ├─────────────▶ Phase 5  Effects (static-first capabilities)
   │                        │
   └──────  Phase 3  Flow (CFG solver + ownership port)
                │
                ├───────────▶   Phase 6  Typestate (automaton lattice + transitionable)
                │
   (all of 1–6 enabled)        Phase 7  Driver (worklist fixpoint + invalidation)
                                              │
                                              └──▶ Phase 8  Comptime (interpreter + reify)
                                              └──▶ Phase 9  Dynamic handlers (optional)

Each arrow: "depends on". Branching phases (4,5,6) are parallelizable after 1–3.
```

Note what this order *does not* do: it does not convert the driver to a
worklist until **after** the first three extensions claim their mechanisms.
That is deliberate, and it is the single highest-leverage scheduling decision in
this document (section 7 explains the reverse temptation and why it is wrong).

---

## 2. Phase 0 — Trust the Harness

**Goal:** Know what "correct" means before touching anything.

- Inventory the gates: `tests/generics.rs`, `tests/monomorphizer.rs`,
  `tests/ownership.rs`, `tests/move_semantics.rs`, `tests/type_checker.rs`,
  and the golden examples under `examples/` (in particular
  `examples/move_semantics/` which encodes the current borrow semantics and
  `examples/generics/` which encodes specialization).
- Add a *behavior matrix* doc (not code): a table of "program → expected
  diagnostic/IR", so each later phase can be judged mechanically.
  **This is [`09-behavior-matrix.md`](09-behavior-matrix.md)** — its §0 is the
  contract, §5 the per-phase flip/add table.
- Set the hook: every phase must finish with this matrix green, plus whatever
  *new* entries the phase itself adds.

**Why first:** because Phases 1–3 are *pure refactors* in disguise, the entire
risk budget is "did we change observable behavior?" Without a matrix, that
question is unanswerable and every refactor becomes a gamble.

---

## 3. Phase 1 — Terms: the Representation Foundation

**Core artifacts:** the term store (arena pool factory for extra node vocab,
overview 3.1–3.2); the `HirType` closed enum (`crates/hir/src/model/types.rs`)
and its `[DedupPoolId<HirType>; 8]` generic array replaced by a **term tree**
with kind + stage fields; the unifier/normalizer extracted from
`crates/hir/src/builders/expression/typing.rs` into a service that extensions
can extend with registered rewrite rules; `substitute_types`
(`crates/hir/src/generics.rs`) re-expressed over terms.

**Deliverable:** *no language change whatsoever.* Every existing program
compiles identically; the per-term kind is computed (everyone is `*`), the
stage flag is `Runtime` for all source values.

**Gating:** full `Phase 0` matrix green. The only new tests added are *internal*
ones (term-tree snapshots, unifier unit cases).

**Why here, why not later:** every other phase consumes representation. HKT
needs kinds *on the term* (tension 2); comptime needs the staging *flag*
(tension 8); traits project *terms* (projection rewriting); the engine
*fights over terms*. Doing this first is the archetypal "pay the plumbing
once" — and because it is semantics-preserving, it is the safest big refactor
in the plan.

**Why not earlier/later:** it cannot be "Phase 0" because it needs the harness
first; it cannot be deferred past Phase 2 because the Engine's fact store is
keyed by terms. This is the *first* of the three dependencies everything hangs
on.

---

## 4. Phase 2 — Engine: Resolution and Caches

**Core artifacts:** predicate registry, fact store, memoized query cache, and
the twin solvers (goal-directed SLD + tabling for existence; bottom-up
semi-naive fixpoint for monotone analysis) over the Phase 1 substrate
(overview 3.3; tension 7 — per-predicate stratifiability metadata).

**First consumers (all re-expressions of hardcoded lookups):**

- `copyable(T)` — replacing `is_copy_type`'s hardcoded `Int/Float/Bool/Str`
  (`crates/hir/src/ownership/mod.rs`) with a fact asserted by the *builtins*
  extension. (This is the move-doc's win, landed early and cheaply.)
- `method(Ty, Name)` — `MethodTable`
  (`crates/hir/src/context/types/methods.rs`) becomes a query result with the
  same fallback semantics (Type's own methods), so traits can *add* entries
  later as *facts*, not table edits.
- `specialize(Fn, Args)` — the monomorphizer's memo `cache` folds into the
  engine's query cache, with `in_progress`/`dead_code` retained as worklist
  metadata (the reachable "frontier" of Phase 7).

**Gating:** matrix green + the three consumers' unit tests (monomorphizer,
ownership, method lookup) passing unchanged.

**Why here:** nothing features-colored is being *installed*; three existing
hardcodes are *moved behind a façade*. It is the lowest-risk placement of the
engine's seams, and it creates the purity+invalidation contract (Salsa-style)
that comptime will eventually lean on.

**Risks:** purity enforcement. Queries that mutate (e.g. a method lookup that
writes an error) must be split into value-returning + diagnostic-emitting
variants before landing, or the cache is unsound the day comptime arrives.

---

## 5. Phase 3 — Flow: the Dataflow Framework and the Ownership Port

**Core artifacts:** the generic dataflow solver (pluggable lattice + transfer
ON function + direction), *graph-provider-agnostic*; a **HIR-statement CFG
provider** (a faithful graph of `HirStatement` blocks with `While`/`If` edges —
today's analyses run on this); the existing IR CFG builder
(`crates/ir/src/cfg/mod.rs`) as the *second* provider, used later for
codegen-side precision.

**Extension action:** port ownership wholesale as *lattice #1* (overview 3.4;
move doc section 7), in two intentionally-easy stages:

1. Same semantics as today (borrow-to-function-end), on the HIR provider —
   every `tests/ownership.rs`/`tests/move_semantics.rs` case green, unchanged.
2. NLL-style, borrow-scoped-to-last-use, on either provider — the deliberate,
   announced diagnostic relaxation (more permissive); matrix updated.

**Gating:** matrix green; the port is invisible to *users* while the solver
proves itself against the hardest test corpus the codebase already has.

**Why here in this order:** it must precede typestate (its contract is a
*query* against phase 3's answers) but it depends on nothing from Phase 2 *in
its core solver* — the two can proceed in parallel after Phase 1; the only
coupling is that `copyable`/`method`/`specialize` (Phase 2) are queried *from*
the transfer functions, so the seams should land Phase 2 *just before* the
flow port needs them.

---

## 6. Phases 4–6 — The First-Extension Wave (parallelizable branches)

### 6.1 Phase 4 — Traits, bounds, associated types

**Extension artifacts** ([`01`](01-traits-and-associated-types.md) section 4):
`trait`/`impl` declaration pools; `trait_ref`/`projection`/`bound_holder`
terms; the `satisfies`/`method`/`normalize` goals over Phase 2; coherence &
orphan & object-safety queries as `well_formedness`; **static dispatch** via
`specialize`, **dynamic dispatch** by reifying per-impl **vtables as data**
(no new IR), and — the tension-6 payoff — lifting
`assert_no_generic_non_functions` for generic aliases and stylesheets, whose
requirements are exactly Phase 4's `satisfies` machinery.

**Why here:** bounds-*less* generics already exist; Phase 4 is the closest
feature to a straight re-application of Phases 1–2 (same recalcitrant shapes:
terms, goals, cache). It also unblocks generic aliases/stylesheets, the oldest
single `unimplemented!` in the corpus (`crates/monomorphizer/src/lib.rs`), at
zero extra theory.

### 6.2 Phase 5 — Effects, static-first

**Extension artifacts** ([`04`](04-effects.md) section 4): `effect`/`handler`
declarations; `effect_row`/`capability`/`handler_ref` terms;
`ambient_capability` goals; driver hooks at creation/use/call (stable
positions, e.g. `ON FunctionCall`); **evidence passing** with `REIFY` erasure
in the statically-known case (the `@capabilities` seed in
`crates/hir/src/model/declarations.rs` finally lives).

**Why here:** needs terms (1), engine (2), driver hooks (positions exist by
Phase 3; the *worklist* is not needed for static-handlers-only programs), and
Phase 4's dictionary approach as its evidence-passing blueprint. It is the
feature most *architecturally* aligned with traits (both are "register a
predicate + reify an artifact"), so it rides the same seams.

### 6.3 Phase 6 — Typestate, on move's shoulders

**Extension artifacts** ([`03`](03-typestate.md) sections 4 & 6): `typestate`
declarations; `state_token`/`protocol` terms; the automaton lattice; transfer
table; and the **`transitionable` contract** — a query the typestate transfer
function *asks of* Phase 3's move answers.

**Why here:** typestate's only true prerequisite is the flow framework *with*
ownership-port (Phase 3) — the contract is a query, so it cannot land before
the move analysis answers queries. It is otherwise independent of Phases 4–5
(a socket automaton in a program with no traits compiles today's object
syntax), hence parallelizable.

**Gating for 4/5/6:** matrix green *plus* each extension's new positive and
negative tests (accepted/rejected programs), which become the `Phase 0` matrix
v2 for the driver conversion.

---

## 7. Phase 7 — The Driver Conversion (the riskiest single step, deliberately last)

**Core artifact:** the worklist + invalidation (overview 3.5.1–3.5.2): the
linear `CompilationStages::build_stages()` becomes `while work: pop; compile;
schedule`. Queries stay pure (Phase 2's seam enforces it); invalidation
contract is exercised, not invented, because Phases 4–6 already *created*
facts (impls, handlers), and re-resolution of `satisfies` after a comptime-less
*addition* has been tested for weeks before comptime ever lands.

**Why last — and why it is *not* cowardice:** the tempting order is "driver
first (it's the 'infrastructure')", but that is backwards under the invariant
of section 1: a worklist driver adds *scheduling* with *zero new semantics* —
it is unremarkable on its own, and everything it permits (re-resolution,
re-synthesis, re-types) only matters once Phases 4–6 have *induced* those
loops organically. Building it first means redesigning it against
nowhere-data; building it now means "we already know the exact re-resolutions
traits/effects/typestate demand; encode *those*." Rework is minimized because
the loop is shaped by *observed* needs, not anticipated ones.

**Risk containment:** the conversion is *behavior-transparent* when designed as
above (it should never change what compiles); the matrix is the safety net; the
two places the loop genuinely changes observable behavior (comptime's
mid-compile additions, dynamic-handler re-resolution) are gated *behind*
their features and their own tests.

---

## 8. Phases 8–9 — The Comptime Payoff and the Optional Dynamic Road

**Phase 8 — Comptime** ([`05`](05-comptime.md) section 4): the staged
interpreter (`eval`) over HIR, `REIFY` onto the driver, the staging law, the
type-as-value story via Phase 1's `kind_of`. Now that Phase 7 exists, comptime
is the *first natural consumer* of the loop, not a leap: "eval a comptime
block" = push a work item; "a comptime-computed type" = a reified term; "a
specialization triggered by comptime" = a `specialize` query. The only new code
is the *tree-walking evaluator itself*, and its own lifecycle tests.

**Phase 9 — Dynamic handlers (optional).** The effects doc's escape hatch
(section 7) — genuinely runtime-chosen handlers — is explicitly *out* of the
core's remit; this phase records that it is a *further extension*, only now
reachable, because (a) it needs the driver loop (Phase 7) for re-resolution of
"which handler is live here", and (b) it needs a real runtime representation
(document the segmented-stack / delimited-continuation discussion; decide
later, never from inside the core).

---

## 9. What "Minimizes Rework" Meant in This Order, and What It Doesn't

The order was chosen to minimize four distinct kinds of rework, in this order
of severity:

1. **Semantic rework** — a pass built for one world-view that has to change
   because a later feature invalidates it. Eliminated by making Phases 1–3
   semantics-preserving refactors (`terms`, `engine`, `flow` are *shape*, with
   zero meaning changes) so no behavioral decision is revisited twice. The one
   genuine semantic change (borrow-to-last-use, Phase 3.2) happens *before*
   any feature depends on the old rule's over-approximation.
2. **Interface rework** — a seam changed after consumers wrote against it.
   Mitigated by landing each seam with a *real* consumer in the same phase
   (`copyable`/`method`/`specialize` in Phase 2; ownership-port in Phase 3).
3. **Enumerative rework** — "touch every pass" for a new feature. Eliminated by
   the reified traversal rule (tension 1): feature terms are *carried*, not
   *matched*, by preexisting passes.
4. **Test rework** — re-baselining goldens because behavior shifted. Confined
   to *two announced* windows: Phase 3.2 (borrow precision) and Phase 7
   (diagnostic re-routing, if any). Everything else preserves the matrix.

**What the order deliberately does *not* claim:** that it is short, or that
intermediate phases are "done" features. Phases 1–3 are compiler-internal and
user-invisible by design; the *first* user-visible feature is Phase 4 (traits).
That is not a failure mode — it is the whole point of making the core moddable
first: the features land *on* it, cheaply, one at a time, after the plumbing
gainfully decides to exist.

---

## 10. Closed-Loop Recap

| Phase | Delivers | User-visible? | Primary risk | Depends on |
|-------|----------|---------------|--------------|-----------|
| 0 | Behavior matrix | no | — | tests as they exist |
| 1 | Terms (tree, kinds, stage, unifier) | no | oldest-big refactor (semantic-preserving) | 0 |
| 2 | Engine (rules, solvers, cache) + 3 façade queries | no | purity enforcement | 1 |
| 3 | Flow solver + ownership port (→ NLL precision) | *slightly* (Phase 3.2 diagnostics) | borrow semantics change is contained | 1, 2 (queries) |
| 4 | Traits (bounds, projections, coherence, disp. strategies), generic aliases/styles | **yes** | coherence/object-safety rules | 1, 2 |
| 5 | Effects, static-first (evidence passing, `@capabilities` alive) | yes | hook positions stability | 1, 2, 4 (blueprint) |
| 6 | Typestate (automaton lattice + transitionable contract) | yes | contract with move latice | 1, 3 |
| 7 | Driver (worklist + invalidation) | no | whole-pipeline shape | 1–6 |
| 8 | Comptime (interpreter, type-as-value, reify) | yes | eval correctness/termination | 7 |
| 9 | Dynamic handlers (optional) | maybe | runtime representation decision | 7 (and a runtime) |

The one-sentence schedule: **make the compiler's shape generic (1–3), let the
first wave of features prove the shape (4–6), then convert the driver to the
loop those features demanded (7), and finally enjoy the loop's marquee tenant,
comptime (8–9) — all while the behavior matrix stays green.**