# Extensible Core — Documentation Series

> **What this series is.** A design study for turning the Slynx compiler from a
> *language compiler* (features hardcoded into its passes) into a *language
> workbench for one language* — a small **core** of mechanisms plus a protocol
> by which every feature (generics, traits, higher-kinded types, typestate,
> effects, comptime, move semantics) is an *installable extension*.
>
> **What this series is not.** Implementation. All code in these documents is
> *illustrative*: pseudocode, manifests, and hypothetical Slynx syntax, written
> to make reasoning concrete. Nothing here is a commit target.

## Reading Map

| Doc | What it settles |
|-----|-----------------|
| [`00-overview.md`](00-overview.md) | The core: arena, terms, engine, flow, driver; the extension manifest; the generics-registers-itself walkthrough; what the core refuses to know. **Start here.** |
| [`01-traits-and-associated-types.md`](01-traits-and-associated-types.md) | Bounds, interfaces, associated types, coherence, static vs dynamic dispatch — as rules and reified data (vtables = structs). |
| [`02-higher-kinded-types.md`](02-higher-kinded-types.md) | Kinds as *structural measurement* in the core; `f<_>`, type lambdas, constructor bounds as a thin extension. |
| [`03-typestate.md`](03-typestate.md) | Flow-sensitive states as one lattice + one transfer table over the core's dataflow framework. |
| [`04-effects.md`](04-effects.md) | User-defined effects, effects on *data*; Koka-style compile-time evidence passing; the `@capabilities` stub gets a future. |
| [`05-comptime.md`](05-comptime.md) | Zig-style comptime: types as values, the staged evaluator, and why it *requires* a worklist driver. |
| [`06-move-semantics.md`](06-move-semantics.md) | Re-packaging today's ownership pass into the flow extension; NLL/Polonius; the `transitionable` contract; `&atomic` as future candy. |
| [`07-tensions.md`](07-tensions.md) | The conflicts between features and the one principle that resolves all of them without the core choosing a winner. |
| [`08-implementation-order.md`](08-implementation-order.md) | The phase-by-phase landing plan, gated by the test matrix, that minimizes semantic, interface, syntactic, and test rework. |
| [`09-behavior-matrix.md`](09-behavior-matrix.md) | **Phase 0 deliverable.** The authoritative table of "program → expected outcome" encoded by the existing test gates; the contract every phase must keep green. |

## Reading Order

If you only read two documents: the **overview** (00) and the **tensions**
(07) — the vision and its honesty.

If you are implementing: the **overview** (00), then the **implementation
order** (08), then the **behavior matrix** (09 — Phase 0's contract that each
phase must keep green), then each feature doc as you reach its phase. Each
feature doc ends with the five questions it answers, so you can verify coverage
by reading any lone doc.

If you are reviewing a specific feature: read its doc first, then the two
documents it cross-references (each doc links its prerequisites inline).

## Recurring Ideas Across the Series

- **Shape, not meaning.** The core operates on hygiene (children + kind) of
  terms, never on feature semantics; feature semantics are registered rules.
- **The reified traversal rule.** Preexisting passes *carry* unknown feature
  terms; they never decide about them.
- **Queries are pure; the driver owns the cache.** Salsa-style invalidation is
  what lets comptime add facts and re-resolve.
- **The IR stays dumb.** Vtables, effect evidence, comptime splices are all
  *data* reified by the driver, never new IR instructions.
- **Extensions compose by query and ordering contract**, never by a
  cross-product interaction table.
- **Nothing new is speculative:** every mechanism in docs 01–06 is harvested
  from an existing file listed in that doc's "what exists" table.