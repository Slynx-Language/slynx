# Typestate as an Extension

> Series: *extensible-core*. Assumes [`00-overview.md`](00-overview.md). Reads
> naturally after [`02-higher-kinded-types.md`](02-higher-kinded-types.md)
> (its section 2 explains why the classic `PhantomData<State>` trick secretly
> required a constructor) and before [`06-move-semantics.md`](06-move-semantics.md)
> (section 4 pins the aliasing contract typestate depends on).

---

## 1. The Feature: Flow-Sensitive Types

Typestate lets a type say *what a value is allowed to do next*, and lets a
*function call change that for the duration of the program's flow*. The
canonical example:

```slynx
let mut sock = Socket::listen(":8080");   // sock : Socket[Closed]   → Closed
sock.connect(addr);                        //                  → Connecting
let stream = sock.handshake();             //                  → Connected
stream.send(...);                          // stays Connected
sock.close();                              //                  → Closed
```

Two properties distinguish real typestate from a runtime enum with a panic:

1. **Flow-sensitivity**: *as the program text is executed forward, the type of
   `sock` changes.* The same variable has different types at different program
   points; at no point is a runtime check emitted.
2. **Linearity-ish alias control**: in-place transitions are only sound if no
   other live reference can observe the intermediate states. This is why
   typestate systems are always built on an ownership/capability substrate
   (detail in section 6).

The brief rejects the common compile-time-counterfeit — `PhantomData<State>`
wrappers — so this doc is about the *real* system.

---

## 2. The Mechanism in Real Systems

### 2.1 Plaid

Plaid (Aldrich et al., CMU) made **states first-class members of types**: a
state is a declaration `state Open { ... }`, states add/remove members, and
methods are annotated with pre/post-states:

```
class Socket {
  state Closed { ... }
  state Open   = Closed + { void send(Data) }   /* Open is Closed plus fields/methods */
  ...
  Socket.open() : Closed -> Open
}
```

The type checker is **flow-sensitive**: after `s.open()`, the variable is
conservatively typed `Socket[Open]` (or a join lower in the state hierarchy, if
control flow merges). Its runtime representation is a *sum of the state-wise
layouts* (a tagged union), because states may add or remove fields.

The piece that makes it compile, and the piece we must keep: Plaid's
**permissions** split values into **unique / shared** aliasing classes.
Transitions are only permitted on *unique* references; a `shared` value cannot
be state-changed, because observing the intermediate state through an alias
would be unsound. Permissions are the *reason* in-place re-typing can exist at
all.

### 2.2 Mungo

Mungo (University of Hamburg) type-checks Java/C-like code against protocol
automata specified in a typestate specification language; its analysis runs
**whole-program over a dataflow graph** — exactly the machinery Slynx is
building in the `flow` primitive (overview 3.4). The internal mechanism is the
one-liner the brief asked for:

> **the "compile-time state flag updated in place"**: for every variable `x`,
> the checker maintains a lattice cell (currently-possible states from the
> automaton). Method calls with protocol effects run the automaton's transition
> function on that cell, in place, in program order; control-flow joins take
> the *greatest lower bound* (most-conservative set) of the incoming cells.

That sentence is the complete spec of a typestate checker, and it is the *same*
sentence that describes definite-assignment analysis ("is this variable
initialized on every path?") and the move checker — one lattice, different
elements, transfer functions, and join.

### 2.3 The connection to Rust's move checker

Rust's ownership checker is often described as novel, but mechanically it is
the classic dataflow framework too: a forward analysis over the MIR CFG with a
finite lattice (per-place: `{uninitialized, live, moved, borrowed-immut,
borrowed-mut, ...}`), transfer functions per statement category (a `move` sets
`moved`; a `&` set `borrowed-*` with a counter; borrow *ends* are inserted by
the NLL region machinery), and join = glb. Polonius then re-expresses the
borrow part in **Datalog** (bottom-up, semi-naive fixpoint — the `flow`/engine
shape of overview 3.3/3.4). The lesson this doc wants to be loud about:

> **Ownership checking, definite assignment, and typestate are one generic
> algorithm with three plug-ins.** Building the framework once (in the core's
> `flow` primitive) and plugging in a *user-configurable lattice* is how
> Slynx gets a *real*, extensible typestate at a price that is no longer "write
> a new analysis subsystem".

---

## 3. What Slynx Already Has

The subsoil is suitable, which is the good news:

- A **place concept**: `HirPlace` — `Variable`, `Field`, `Index` (arrays),
  `Deref`, `Temporary` — built from any expression in
  `crates/hir/src/ownership/place.rs`. Typestate needs per-place cells; the
  place vocabulary exists and is sound (temporaries are distinct from places).
- A **working dataflow-ish pass**: `crates/hir/src/ownership/`, with
  `PlaceState { borrowed_mut, borrowed_immut, moved }`
  (`ownership/state.rs`) and `ExpressionUse { Read, Move, Borrow, BorrowMut }`
  (`ownership/mod.rs`). It is a forward walk over `HirStatement`
  (`crates/hir/src/model/statements.rs`) and `HirExpressionKind`
  (`crates/hir/src/model/expression.rs`) with no CFG, no fixpoint, and no
  borrow *end* (`release_borrow` in `state.rs` exists but is never called —
  borrows live to the function's end).
- A **CFG builder**: `crates/ir/src/cfg/mod.rs` already builds a petgraph
  `StableDiGraph<BasicBlock, EdgeKind>` with real edge kinds (conditional,
  backedge, exit). *The graph exists; the solver does not.*

The gap is exactly one abstraction: a generic `Lattice` value + per-instruction
transfer functions + a runner that iterates to a fixpoint. That missing piece
is the *core*'s `flow` primitive, not a typestate feature.

---

## 4. The Extension Design

```text
extension "typestate":
  order: 70            # after move-analysis (its permission contract) and hkts

  declaration_kinds:
    typestate  ::= data(name, state_set, transition_set, entry_state, capabilities)
                  # states: named; transitions: (callable-name × from-state × to-state...)

  type_terms:
    state_token ::= arity: 1   kind Type   # any value-typed term, plus a "current state" claim
    protocol    ::= arity: 2   kind Type   # attach an automaton record to a type

  flow (lattice registration):
    lattice: per-place cell = subset of automaton states (bottom ⊆, top = all states)
    transfer rules:
      call f(args) where f is a transition:  set(place, δ(state(place), f))
      assignment to x:                       set(x, all-states)   # conservative re-init
      control-flow merge (join):             glb of incoming cells
      `let x = con` of a stateless value:    unchanged (x not in automaton's type)

  engine (querying, not transfer):
    state_of(place, block) -> LatticeCell       # what the analysis proved
    transition_allowed(f, from) -> bool         # is f legal in this state?
    invariant_ok(place, invariant) -> bool      # user-declared, checked per block
```

And the *language surface* (illustrative — no keywords exist today; `typestate`
is reserved nowhere):

```slynx
typestate Socket {
    state Closed;
    state Listening;
    state Connected;

    entry Closed;

    transition open(addr: Addr)      : Closed    -> Listening;
    transition handshake()           : Listening -> Connected;
    transition connect(addr: Addr)   : Closed    -> Connected;
    transition send(data: bytes)     : Connected -> Connected;   // self-loop = legal in-state call
    transition close()               : Connected -> Closed;
    // no transition on state Listening → close() is *statically* rejected there
}

object Socket { ... }

func use_socket() {
    let mut sock = Socket { url: "..." };   // state of sock := Closed (entry)
    sock.open(addr);                        // type-level: Closed → Listening
    let stream = sock.handshake();          // Listening → Connected
    sock.send(data);                        // ok (self-loop)
    sock.close();                           // Connected → Closed
    // sock.send(data);                     // REJECTED: no transition out of Closed
}
```

Compilation, mechanically, is four steps that the reader can now describe
without new vocabulary:

1. **Build** — `typestate`/`transition` records land in new arena pools; the
   `protocol` term binds the automaton's record id to the object type.
2. **Flow** — the framework instantiates fresh per-place lattice cells at the
   entry of the function, seeded with the entry state, and runs the generic
   solver (transfer + join + fixpoint). The only typestate-specific code is the
   transfer table mapping callable → δ, and the join (glb).
3. **Check** — the engine answers `transition_allowed(f, from)` per call site;
   a rejection is a compile-time diagnostic at the call, not a runtime check.
4. **Erase** — the IR is untouched: `IRType` stays fully concrete, no state tag
   is ever emitted (states change no runtime layout here — a deliberate,
   documented simplification vs. Plaid, which allowed per-state fields).

---

## 5. States That Change Layout (the Plaid question, and our answer)

Plaid allowed states to *add and remove fields*, so `Socket[Open]` had a
different runtime layout than `Socket[Closed]`, and the value was a tagged
union over state layouts. Mungo's flavor keeps one layout and checks the
protocol only.

Slynx's recommendation: **state ~= protocol-carrying enum tag, not a new
layout.** Add `state X { ... }`-style *member* differences later only if a
concrete use demands it. Reasons:

- The existing IR has `Union`/`Struct` descriptors (`crates/ir/src/types/
  irtype.rs`) and a real enum story (`crates/codegen/src/lib.rs` implements
  `EnumLayout`); a "state as enum-tag + layout switch" is representable *today*
  if ever needed, via ordinary types — i.e., it is an *encoding*, not a feature.
- Layout-changing states cost every consumer (codegen, layout, monomorphizer,
  defaults) for a benefit (memory reuse) the language has not yet asked for.

The doc's honest position: by **not** tying typestate to layout, we keep the
extension a pure *analysis* extension (one lattice, one transfer table), which
is the difference between a weekend extension and a rewrite.

---

## 6. The Aliasing Contract: Why This Needs Move Analysis Underneath

In-place re-typing is only sound while no other live reference can observe the
state *between* transitions. The failed states of "naive typestate" are exactly
octopus-aliasing bugs: `let s2 = &sock; sock.connect(); s2.send(...)` — if both
see the value, which state is it in? Plaid solved it with **permissions**;
Mungo with whole-program alias-precision. Slynx's substrate already has the
right tool, and it is the subject of
[`06-move-semantics.md`](06-move-semantics.md):

- The move extension's lattice provides "is this place uniquely owned / can it
  be moved / is it borrowed" per point (overview 3.4; `crates/hir/src/
  ownership/` is today's seed).
- **Contract (pin this in the tensions doc [`07`](07-tensions.md) section 4):
  a place may *only* be re-typed by the typestate transfer function when the
  move analysis reports it as `moved`-free *and* `borrowed`-free at the line of
  the transition.** If a borrow is live across a state transition, the check
  fails: *"cannot transition `sock` while borrowed."*
- Because both analyses are the *same* framework (flow) with different
  lattices, they run as **ordered passes — move first, typestate second** — and
  the typestate transfer function *queries* the move pass's result, per the
  "analysis feeds forward by lattice" rule (overview 7.1). No special-casing, no
  cross-product registry.

---

## 7. Alternatives Considered and Refused

1. **`PhantomData<State>` / session-typed wrapper types.** Refused *for the
   brief's own stated reasons*: it does not *change types* but *hides them*;
   every operation is a `consume`+`return` shuffle; it reads as ceremony, not
   as a state machine. (And, per the HKT doc section 2, it secretly required a
   constructor anyway.) The one genuine merit — it lives inside ordinary type
   inference — is not worth the ergonomic tax.
2. **Session types (Haskell / linear-coroutines / `lchannels`).** The
   mathematically strongest flavor (protocols as types, duality, linearity
   guarantees). Refused as a *core modeling*: it needs full linear typing as an
   ambient discipline, whose machinery Slynx is *not* committing to everywhere;
   the audience (a UI/productivity language) does not want protocols expressed
   as monadic channel choreographies. The tensions doc keeps session types
   alive as a *future* extension that *stands on* the same flow framework.
3. **Runtime enforcement only (assert-state enum + panic).** Refused: the whole
   point is compile-time rejection; runtime checks contradict the "no runtime
   check is emitted" promise in section 1, and panic paths are untestable at
   compile time.
4. **Whole-program alias precision as a first-class extension (Mungo-style).**
   Not refused so much as *deferred-to-under*: the move/ownership extension
   gives us enough alias precision (unique, borrowed) to make transitions sound
   for the overwhelming majority of programs; upgrading the alias model later is
   *strengthening a lattice*, not a new feature.

---

## 8. Compatibility Fallout

- **Errors move earlier or change text.** `Socket.send` on a closed socket is
  today a nonexistent-method error (method-not-found); with typestate it is a
  *state* error ("`send` is not available in state `Closed`"). Diagnostic
  baselines in `tests/type_checker.rs` may shift; messages are preserved via
  the `QUERY-MISS` refinement hook (overview 5.2).
- **The ownership pass must relocate onto the flow solver *first*.** The
  current forward walk structurally visits statements; the framework pass
  visits the CFG. The migration plan is in [`06`](06-move-semantics.md)
  section 5 (and [`08`](08-implementation-order.md) schedules it as a staged
  work item) so typestate *never* runs on the old machinery.
- **Programs that abuse aliasing around transitions now fail to compile** where
  the old code was silent. This is a *feature*, but it is user-visible: the
  language blog post for v-next should say "borrows are no longer allowed to
  cross a state transition."

---

## 9. The One-Sentence Architecture

Typestate is **one lattice** (a per-place subset of automaton states), **one
transfer table** (method call ⇒ automaton δ), **one join** (glb), and **one
permission contract** (no live borrows across transitions) — all registered
into the core's flow framework, which already has a graph and will already own
the move lattice. The *feature* is a weekend's manifestation; the *framework*
is the engineering.

---

Next: [`04-effects.md`](04-effects.md) — user-defined effects and why the existing
`@capabilities` attribute is the seed of the whole story.