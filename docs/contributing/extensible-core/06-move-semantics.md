# Move Semantics and Single-Writer as an Extension

> Series: *extensible-core*. Assumes [`00-overview.md`](00-overview.md). This is
> the *re-packaging* document: the feature already exists
> (`crates/hir/src/ownership/`), and the job is to (a) explain its current
> mechanism honestly, (b) make it an extension over the core's `flow` framework,
> and (c) pin the contract that typestate (doc 03) stands on.

---

## 1. Where We Are Today (the honest inventory)

Slynx has a working **ownership checker**, described by its own doc comment in
`crates/hir/src/ownership/mod.rs` as "similar to Rust's ownership system but
simplified." Let me be precise about what it actually does, because the
extension design is a *faithful translation* of these facts:

- **Runs on HIR statements, per function, forward, structurally.** It is a
  walk over `HirStatement`/`HirExpressionKind`
  (`crates/hir/src/model/statements.rs`, `expression.rs`) in source order —
  *not* a fixpoint over a CFG (`crates/ir/src/cfg/mod.rs` exists but the
  checker never touches it).
- **Lattice is hardcoded.** `PlaceState { borrowed_mut: u8, borrowed_immut: u8,
  moved: bool }` in `ownership/state.rs`; `ExpressionUse { Read, Move, Borrow,
  BorrowMut }` in `ownership/mod.rs`. `BorrowKind { Mutable, Immutable }`. The
  *elements* are fixed by the code, not pluggable.
- **Copy-ness is hardcoded.** `is_copy_type` special-cases `Int`, `Float`,
  `Bool`, `Str`. There is no user-definable "this type is `Copy`" — a real
  limitation, and the first thing the extension lifts.
- **Borrows never end.** `release_borrow` exists in `state.rs` but is never
  called; a borrow live to the end of the function is a *sound
  over-approximation* for a linear walk. (It is also why `let r = &x; print(r);
  print(&x);` is stricter than necessary — borrow lasts till function end —
  which is exactly the NLL story, minus the solving.)
- **Codegen consumes the analysis.** `LoweringState.ownership` in
  `crates/codegen/src/lowerers/mod.rs` turns the analysis into `Move`/`Copy`
  opcodes in the IR (`crates/ir/src/model/instruction.rs` — that `Move` opcode
  is documented as "mainly idealized to make it easier to represent move
  semantics"). The IR is the *dumb* record of the analysis; the analysis is
  the brain, and the brain is inside the HIR subsystem.

That is a real, usable subsystem. It is also *precisely* the "once-and-only"
dataflow problem (definite assignment / use-after-move) that Polonius and
Mungo both perform with a generic framework. The claim of this doc is that the
*entire* feature survives a move into the core's `flow` framework as **one
registered lattice + transfer functions + query-backed copy predicate — plus
the Polonius-style rules for lending**, and that every other extension in this
series (typestate above all) quietly depends on it.

---

## 2. The Mechanism in Real Systems

### 2.1 Rust NLL & Polonius: from dataflow to Datalog

Rust's borrow checker, as it stands today, is **two subsystems layered**:

- **The *dataflow* layer** tracks, per program point, for each place, which of
  `{definitely-initialized, live, moved, borrowed-immut, borrowed-mut}` holds.
  This is the classic forward lattice (transfer functions per statement,
  join = glb), iterated to fixpoint over the MIR graph. "Use-after-move" and
  "assignment to moved value" are its vocabulary.
- **The *region* layer** assigns to each borrow a *region* (a set of program
  points from the borrow's start to its last use). NLL computes regions by
  *inference* over the graph (a constraint problem over region variables);
  Polonius re-expresses the same problem in **Datalog**, with three fact
  bases (`origin_live_on_entry`, `loaned_to`, `borrowed_from`) and a handful of
  monotone rules (e.g. a borrow is *live* at a point iff its region is live and
  its dataflow facts hold; conflicts fire when a move or a `&mut` coexists with
  a live borrow). Polonius's implementation is literally a **bottom-up,
  semi-naive fixpoint** over those rules (*the* shape the overview calls the
  bottom-up engine, section 3.3) built on the `datafrog` Datalog engine.

The reason to study Polonius, and not just the resource-constrained NLL, is the
architectural lesson: **the borrow rules are *rules*.** They are separated
from lattice transfer functions, expressed declaratively, and *re-runnable*.
That separation is exactly what the core's engine (registered predicates +
clauses) makes available to Slynx for free.

### 2.2 Pony: the capability model (the road not taken, reason recorded)

Pony rejects dataflow analysis entirely. Each type carries a **reference
capability** — `iso` (isolated, unique), `val` (immutable, may share),
`ref` (mutable, may share but not escape), `box` (read-only), `tag` (identity
only) — and **viewpoint adaptation** derives the capability of any
projection `a.b` from the capability of `a` plus the field's declared
capability (a small subtyping/meet table). Ownership is a *property of types*,
checked by plain type rules, with **no per-point analysis at all**.

The trade is stark and it is why Slynx refuses the full switch (section 6):
capabilities make the *whole type system* carry ownership (every type, every
field, every match arm), they reject some programs that NLL-style dataflow
would accept (a carefully-scoped temporary reuse pattern), and — critically
for this language — they make **typestate's in-place transitions** harder to
express (a capability is *frozen by the type*, not decided per program point).
What Pony does well — and what Slynx should *steal* — is the vocabulary: a
*small set of named capabilities* (unique/immutable/...) as user-facing
concepts, even though our *mechanism* is dataflow, not type-level capability
inference.

---

## 3. The Extension Design

```text
extension "move":
  order: 50            # lowest of the flow extensions: every other flow
                       # analysis (typestate) queries its outputs

  engine (predicates — replaces is_copy_type, and its cousins):
    copyable(T)   :- builtins ext asserts Int/Float/Bool/Str are copyable
                  :- (future) user `copy` trait/impl terms registered here
    movable(T)    :- not copyable(T)      # the default, dataflow-driven

  flow (the ownership lattice + transfer laws):
    lattice element per place:
       { Uninit, Live, Moved, BorrowedImmut, BorrowedMut }, with a
       "region" annotation per borrow (the NLL zone: start..last-use).
    transfer rules (instruction class keyed):
       Read  of copyable place : unchanged
       Move  of non-copyable   : set Moved on source, Live on destination
       Borrow(k)               : set Borrowed(k) + region = [here, last-use]
       Assign to x             : set Live (kill old state)
       Call with arg           : consumes iff movable (the ExpressionUse story)

  engine (bottom-up rules — the Polonius half, declared not hardcoded):
    borrow_live(p, point)        :- region(point) ∧ both(d1, d2)
    conflict_move(p, point)      :- Moved(p) ∧ Borrowed*(p) at point
    new_escape_region(...)       (the "&atomic" extension hooks here, section 5)

  driver hooks:
    AFTER-TYPECHECK per declaration:  run the ownership solver; record answers
    ON FunctionCall:                   the per-call move/borrow decision
    REIFY:                             move/copy opcode choice to the IR
```

The feature is thus *two registered mechanics* conjoined:

1. **A dataflow lattice** (`flow`) that answers "what is each place doing at
   each block" — a faithful generalization of today's `PlaceState` walk, with
   the same sound semantics and *better* precision (borrows now *end* at last
   use, because the solver runs over the CFG, not source order).
2. **A Polonius-style rule set** (`engine`, bottom-up) that turns "may a move
   coexist with a live borrow" into *declared facts*, so the future `&atomic`
   capability (section 5), and typestate's alias rules, are *new rules* and
   *new facts*, not new match arms.

---

## 4. The Contract Typestate Depends On

[`03-typestate.md`](03-typestate.md) section 6 stated it and pinned it; this
doc owns it. Typestate can only *re-type a value in place* when no aliases can
observe the intermediate state. The contract, encoded as rules in *this*
extension:

> **`transitionable(place, point)` holds iff `move` answers, at `point`, that
> `place` is `Live` with zero live borrows (`borrowed_immut == 0 &&
> borrowed_mut == 0`) at every step inside the transfer.**

The typestate transfer function *queries* this predicate. Because both analyses
are separate lattices in the same solver, the composite is *ordered* — the move
pass runs first, typestate second, per the overview's "analysis feeds forward
by lattice" rule (overview 7.1) — and there is **no cross-extension
interaction table**: the contract *is* a query; the query *is* the
interaction.

---

## 5. The Concrete Win: User-Definable Capabilities (`&atomic` and friends)

Today `is_copy_type` is closed. The extension opens it by making copy-ness a
predicate — but the *mechanism* for borrows opens something bigger: a
**third borrow kind** (`&atomic T`) becomes:

```text
extension "atomic_borrow"  (or a later builtin):
  registers a new BorrowKind-like element { BorrowedAtomic }
  registers transfer laws: read-through is always ok (relaxed), write-through
     requires BorrowedAtomic only, no other borrow coexists
  -- the checker needs ZERO core changes: atomicity is a lattice element and
     two rules, not an Opcode or a match arm.
```

That is the extension thesis in one picture: **the hardest part of Rust's
ownership (borrow-kinds and their interplays) is, in this architecture, a
small, named, registrable rule-set.** What took Rust three RFCs to re-engineer
(NLL) and an entire Datalog project to formalize (Polonius) Slynx pre-buys by
putting the *framework* in the core and the *rules* in the extension.

---

## 6. Alternatives Considered and Refused

1. **Full Pony reference-capability reform.** Refused as the *theme* for the
   reasons in section 2.2 — in particular it abdicates NLL-style precision and
   blocks typestate's in-place story. **Adopted as the *vocabulary***: users
   *think* "unique" / "immutable" / "read-only", and future capabilities wear
   Pony-ish names, while the machine underneath stays dataflow + rules.
2. **Keeping the hardcoded `is_copy_type` and extending it occasionally.**
   Refused: an open language must let library authors define
   copy-value shallow types; a *predicate* (with the builtins ext supplying the
   current `int/float/bool/str` cases) keeps that future a one-line registration
   instead of a compiler edit in `crates/hir/src/ownership/mod.rs`.
3. **Mungo-style whole-program alias precision as a first-class feature.**
   Not refused — deferred. The borrow-region precision already delivered by
   NLL-style regions covers the alias-sensitivity typestate realistically needs;
   a *whole-program* precise aliasing extension would be additive (stronger
   rules, same lattice plumbing), so "not now" costs no architecture.
4. **Lifting ownership *out* of the "similar to Rust" assumption entirely** —
   a per-declaration "checked region calculus" à la Rust-principal-focus only.
   Refused: every consumer (codegen `Move`/`Copy`, the current tests
   `tests/ownership.rs`, `tests/move_semantics.rs`, examples under
   `examples/move_semantics/`) depends on the existing, Rust-like, per-function
   model. The extension keeps the model and *generalizes the seems*, exactly
   what a low-risk repackaging should do.

---

## 7. Compatibility Fallout (the CFG boundary is the real change)

Two decisions are in this bin, and they are the *only* user-visible ripples in
the whole series, so the docs are unusually specific about them:

1. **Where analysis runs: the recommended boundary is a post-specialization
   CFG IR**, because (a) place precision wants concrete types (an `enum`'s
   discriminant moves and `doubles` vs `i32`-payload moves differ), and (b) the
   solver needs CFG edges (`crates/ir/src/cfg/mod.rs` already builds them). The
   current checker runs pre-specialization on HIR — the *diagnostics* it emits
   have lived in `tests/ownership.rs`/`tests/move_semantics.rs` and the
   examples. The migration (scheduled in [`08`](08-implementation-order.md)
   section 5) is staged:
   - *Stage 1:* port the existing HIR walk verbatim onto the flow framework's
     lattice + solver, with the *same* semantics (borrow-to-function-end),
     so every existing test passes unchanged.
   - *Stage 2:* switch the solver's source to the CFG IR and enable
     borrow-scoped-to-last-use (the NLL upgrade). Diagnostics *can* change
     (previously-accepted programs that borrow past their last use now... stay
     accepted — this direction is *more permissive*); the test churn is
     bounded, deliberate, and announced.
2. **Codegen consumes the new answer path.** `LoweringState.ownership`
   (`crates/codegen/src/lowerers/mod.rs`) reads the analysis through the query
   layer, not by holding a struct; the `Move`/`Copy` IR opcodes are unchanged —
   the IR was deliberately designed dumb, and it stays dumb.

There is also a *good* ripple: the hardcoded `release_borrow`-never-called
rule dies, and the "borrows last to end of function" over-approximation becomes
*precise* for free, which unblocks patterns like temporary-scoped refs that the
current checker (and current tests) disallow.

---

## 8. Summary

- **Gets repackaged:** Today's HIR ownership walk becomes a `flow` lattice
  (with borrow regions) + a query-backed `copyable` predicate + a Polonius-style
  `borrow_live`/`conflict` rule-set. The feature's *semantics* are identical in
  the default; its *precision* and *extensibility* improve.
- **Gets unbought:** The core no longer knows `Copy`/`Move` (a registered
  predicate), `int/float/bool/str` copy-ness (a builtins-ext rule), borrow-kinds
  (a lattice vocabulary), or opcode choice (a query result). The IR keeps
  emitting `Move`/`Copy`, because that's what backends want — the *decision*
  simply stopped being core-internal.
- **Pays the rent:** typestate's alias contract (section 4), the `&atomic`
  future (section 5), and the eventual whole-program precision (section 6.5) all
  stand on this extension's answers. It is the *foundation* of the flow story,
  which is why the impl-order doc puts its port first among the flow work.

---

Cross-references: typestate contract (`03` section 6); flow primitive (overview
3.4); bottom-up engine (overview 3.3); comptime's staging flag meeting the
`copyable` predicate (overview 3.2); driver's analysis ordering rule
(overview 7.1).

Next: [`07-tensions.md`](07-tensions.md) — the honest map of what conflicts with
what, and how the core resolves each conflict without choosing a winner.