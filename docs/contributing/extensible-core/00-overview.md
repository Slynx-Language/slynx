# The Extensible Core: Architecture Overview

> This is the first document in the *extensible-core* series. It defines what the
> **core** of the Slynx compiler should be: the minimal set of mechanisms that
> makes every language feature — generics, traits, higher-kinded types,
> typestate, effects, comptime, move semantics — an *installable extension*
> rather than a hardcoded part of the compiler.
>
> The other documents in this series detail each feature as an extension:
>
> - [`01-traits-and-associated-types.md`](01-traits-and-associated-types.md)
> - [`02-higher-kinded-types.md`](02-higher-kinded-types.md)
> - [`03-typestate.md`](03-typestate.md)
> - [`04-effects.md`](04-effects.md)
> - [`05-comptime.md`](05-comptime.md)
> - [`06-move-semantics.md`](06-move-semantics.md)
> - [`07-tensions.md`](07-tensions.md)
> - [`08-implementation-order.md`](08-implementation-order.md)
>
> Titles are abbreviated in the text below as *the overview*, *the traits doc*,
> and so on.

---

## 1. The Problem We Are Solving

Read any modern language compiler and you will find the same archeology. The
first version hardcodes the features the author cares about. Each new feature
touches *every* pass: the parser grows a new AST node, the type checker grows a
new case, the lowering pass grows a new arm, the error printer grows new
messages. Ten features in, the compiler is a pile of orthogonal `match`
statements that all enumerate the same closed universe of "what the language
is".

Slynx is today exactly at that point, and it is a *good* place to stop and
redesign. The current codebase is small, clean, and honest about its
limitations. The pipeline is a straight line, and every stage enumerates, by
hand, the current feature set:

```
lexer ──▶ parser ──▶ module_loader ──▶ hir build ──▶ ownership ──▶ monomorphizer ──▶ codegen ──▶ SlynxIR
        (src/lib.rs, src/compilation_context/mod.rs: CompilationStages::build_stages)
```

Already the seams are visible. Every stage needs to keep in sync with a set of
**closed enums** that are really lists of features:

| Enum | File | What it enumerates |
|------|------|--------------------|
| `HirType` | `crates/hir/src/model/types.rs` | Every type form in the language today |
| `HirExpressionKind` | `crates/hir/src/model/expression.rs:310` | Every expression form |
| `HirStatement` | `crates/hir/src/model/statements.rs` | Every statement form |
| `AnyLocalDeclarationId` | `crates/hir/src/id.rs` | Every declaration kind: Object, Function, Component, Style, Alias, Static, Enum |
| `IRType` | `crates/ir/src/types/irtype.rs` | Every *lowered* type, already strictly concrete |
| `Opcode` | `crates/ir/src/model/instruction.rs` | Every instruction, hand-tuned per feature |
| Attribute kinds | `crates/hir/src/model/declarations.rs` (`HirAttributeKind`) | Builtin, Capabilities, Unknown |

Look at two of them closely, because they are the whole argument in miniature.

`HirType` is a closed enum. Generics are already *in* it — as `Reference` and
`GenericComponent` and `GenericParam`. When somebody adds traits, they will have
to extend this enum and then fix every `match` that touches it:
`crates/hir/src/helpers/views/types.rs` (rendering names), the type checker
(`crates/hir/src/builders/expression/typing.rs`), the monomorphizer, the
codegen's `TypeLowerer` (`crates/codegen/src/lowerers/mod.rs`), the IR
`IRType`... The list is long and grows with every feature.

`HirAttributeKind` already contains a *stub* for a feature that does not exist
yet:

```rust
pub enum HirAttributeKind {
    Builtin { name: SymbolPointer },
    Capabilities(Vec<SymbolPointer>),   // <- "fs", "io", "net" parsed, nothing consumes it
    Unknown,
}
```

`@capabilities("fs", "io", "net")` is accepted by the attribute pipeline
(`crates/hir/src/builders/attributes/mod.rs`), stored, and then forgotten.
This is the pattern we want to *institutionalize*: the compiler meeting a
feature's syntax before the feature's semantics exist. Today it is an accident.
We want it to be a design.

The design is simple to state and hard to do well:

> **The core knows nothing about features.** It provides the mechanisms a
> feature needs — representation, resolution, analysis, transformation,
> scheduling — and a protocol by which features register themselves into those
> mechanisms.

Everything the language *is* (traits, HKT, typestate, effects, comptime, even
move semantics) lives in extensions. The core is a **language workbench for one
specific language**, not a compiler for a fixed one.

---

## 2. The Analogy: A Game Engine, Not a Game

A game engine does not ship with a game. It ships with *entities*, *components*,
*systems*, an *event loop*, and a fixed order in which systems run. The game
(whether it is a racing game or a platformer) is nothing but a collection of
systems that read and write the same world.

- *Entities* are not a game feature; they are identity.
- *Components* are not a game feature; they are data slots attached to
  identity.
- *Systems* are not a game feature; they are code that runs on a schedule and
  is told which entities it cares about.
- The *event loop* is not a game feature; it is the rhythm.

The compiler core should map this structure directly:

| Game engine | Compiler core | What it is |
|-------------|---------------|------------|
| Entities | **Arena identities** | Stable pool ids for types, declarations, expressions, instructions |
| Components | **Interned terms** | The shared, extensible representation of types, kinds, and constraints |
| Systems + query registry | **Resolution engine** | Goal-driven proof search + bottom-up fixpoint analysis, over registered rules |
| Event loop | **Driver / worklist** | Decides *what to compile next*, runs passes, caches and invalidates queries |
| Fixed system order | **Analysis/lowering pipeline** | The peephole through which every feature passes, with well-defined hooks |

The game (traits, effects, comptime...) registers *systems* into the engine. It
never edits the engine. "Giving the engine a spell system" is an extension; the
engine just provides "systems plus scheduled execution plus shared memory".

The subtle but crucial inversion: **today the compiler is the game.** The
compiler *is* generics. *Is* ownership. *Is* components. The redesign makes the
compiler a shell that a game can be written against.

---

## 3. The Core's Five Primitives

The core is five subsystems, plus one contract. They are described in detail
below; the short version first:

```
┌────────────────────────────────────────────────────────────────────────────┐
│                                  THE CORE                                  │
│                                                                            │
│  1. ARENA      stable ids, interned pools, dedup   (mostly exists today)   │
│  2. TERMS      extensible, kinded, staged type terms + unification         │
│  3. ENGINE     goal-driven prover + bottom-up fixpoint over registered     │
│                rules (the only place proofs exist)                         │
│  4. FLOW       CFG IR + pluggable-lattice dataflow solver                  │
│  5. DRIVER     worklist scheduler, query cache, amplification/transform    │
│                hooks, reification                                           │
│                                                                            │
│  extension contract: the MANIFEST (section 6) — who registers what,        │
│  where, and in what order                                                   │
└────────────────────────────────────────────────────────────────────────────┘
```

Each primitive has three faces, documented in each section:

1. **What it is** — the concept.
2. **What already exists** — concrete files, so we never redesign what works.
3. **What must change** — the delta between today and the goal.

---

### 3.1 The Arena — identity and storage

**What it is.** Every node in the compiler — a type, a declaration, an
expression, an instruction, a constraint — lives in a dense, append-only pool
behind a stable id. Nodes never contain other nodes; they contain ids. This is
already the dominant style in this codebase, and it is worth keeping and
standardizing.

**What already exists.**

- `common::pool` — `Pool`/`PoolId` and `DedupPool`/`DedupPoolId`
  (`crates/common/src/pool/mod.rs`, `crates/common/src/pool/id.rs`).
- `HirStore` (`crates/hir/src/store.rs`) — pools for expressions, statements,
  component expressions, places, plus `files`, `lang_items`, and variable-name
  interning.
- `TypeStorage` in `crates/hir/src/context/types/mod.rs` — the type pools used
  while building HIR: dedup pools for `HirType`, plus per-feature pools.
- `SlynxIR` (`crates/ir/src/ir.rs`) — flat append-only `Vec`s for `globals`,
  `functions`, `components`, `labels`, `instructions`, `impure_instructions`,
  `types`, `strings`. Every `Value` is just an index into `instructions`.

**What must change.** Mostly attitude, not machinery. Two concrete deltas:

1. *Every* feature-sized concept must get a pool. Today `TypeStorage` has a
   fixed set of pools that are named after current features (`types`,
   `structs`, `enums`, `components`, `styles`...). The arena layer must offer a
   *pool factory* so an extension can say "give me a pool of `TraitImpl`s" and
   get the same interned, deduped storage the core uses. The registration
   protocol (section 6) is what hands these pool slots out.

2. *IDs must carry their kind.* `PoolId<T>` is currently one fresh type per
   node kind (`DedupPoolId<HirType>`, `PoolId<HirExpression>`, ...). When the
   type system becomes an extensible tree (section 3.2), the core needs a
   uniform handle that says "this is a type term, and here is its node kind" —
   so passes written before a feature existed can still *walk* a tree they have
   never seen (see the "reified traversal" rule in section 5.2).

The arena primitive is the least glamorous and the most load-bearing. Every
other primitive names its data by arena ids. If the arena layer is solid, the
rest of the core never manages memory, alias lifetimes, or hash-map keys: it
manages numbers.

---

### 3.2 Terms — the extensible, kinded, staged type representation

**What it is.** Today `HirType` is a closed enum whose variants *are* features:
`Reference { generics: [DedupPoolId<HirType>; 8] }`, `GenericComponent`,
`GenericParam { index, name }`, plus `Int`, `Float`, `Bool`, `Str`, `Void`,
`Nullable`, `Array`, `Vector`, `ImutableRef`, `MutableRef`. The `Reference`
variant's generic parameters are a fixed `[DedupPoolId<HirType>; 8]` array
(`crates/hir/src/model/types.rs:360` — the doc comment says "in `Vec<int>`,
this would contain `[int]`") — an arbitrary ceiling that exists only because
the array is fixed-size.

The core term primitive replaces this with a *tree of node terms over an
extensible node-vocabulary*, where every node is an arena id. The node
vocabulary has exactly two parts:

- **Core nodes**, which every extension may build on: `apply` (type
  application), `tuple`, `fn`, `data` (a struct/enum/union descriptor id),
  `ref` (borrow/lifetime marker), `primitive`, `var` (a type variable), and
  `hole` (an unsolved metavar during inference).
- **Extension nodes**, opaque to the core except for three contracts: they
  carry child term ids, they have a *kind* (see below), and they can provide a
  descriptive name for diagnostics.

That means `Trait<Self>` is not "a variant". It is `apply(trait_ref, var)`,
where `trait_ref` is an extension node registered by the traits extension. The
differences that matter:

- The core's unifier (below) can unify or walk **any** tree, because walking
  only needs "children + kind", both of which every node — core or extension —
  provides.
- The type checker written for generics can *seize up* on a trait application
  it does not understand, but it can still *render it*, *substitute into it*,
  and *keep it in a map*. That alone removes most of the "touch every pass"
  burden.

**Kinds.** Every term carries a *kind*, which is a second-level type. The
minimal set the core knows: `Type` (the kind of ordinary types — written `*`),
`Fn(kind, kind, ...)` (the kind of type constructors — `* -> *`), `Universe`
(the kind of kinds), and extension kinds are barred: the core's kind system is
closed because kinds are the *structural plumbing* of the term tree, not a
feature. Section 3 of the HKT doc ([`02`](02-higher-kinded-types.md)) argues
this in full. The shorthand version:

> The core does not know "Functor". It knows that a term has a shape with an
> arity, the way a parser knows a call has arguments. That arity *is* the kind.
> Higher-kinded *user-facing* syntax (`f<_>`), type lambdas, and quantification
> over constructors are the HKT extension; the *measurement* of arity is core.

**Staging.** Every term can additionally be marked with a *stage*:
`CompileTime` or `Runtime`. This is the seed of comptime and of effects and of
trait-object generation (all three use "values/types known only inside a
particular phase"). The core treats stage as a flag with two legal values and
one rule: *a value of compile-time stage may be spliced into the program
(section 3.5, reification); a value of runtime stage may not be used where a
compile-time value is required.* The comptime extension owns the *semantics* of
staging (the interpreter, section 3 of [`05`](05-comptime.md)); the core owns
only the *flag* and the *rule*.

**Unification & normalization.** The core provides one unification service over
terms, with occurs-check, and one normalizer that applies registered
reduction/rewrite rules. Unification does not know what a "projection" is, but
it *applies* projection rules the traits extension registers (section 6.2 of
this doc walks the mechanism). This service replaces the hand-written recursive
`unify_types` today in `crates/hir/src/builders/expression/typing.rs`.

**What already exists to harvest.**

- `HirType` / `DedupPoolId<HirType>` in `crates/hir/src/model/types.rs`.
- `TypesContext` and `TypeRegistry` in
  `crates/hir/src/context/types/{mod,registry}.rs` — name→type-id maps.
- `substitute_types` and the whole `GenericTypeArguments` machinery in
  `crates/hir/src/generics.rs` — a working substitution service over the
  current theory; it must be re-expressed over terms without losing it.
- Struct-descriptor pools (`structs`, `enums`, `unions`, `components`, `styles`)
  in `TypeStorage`.

**What must change.** The closed enum becomes a core-node shell plus a slot for
extension nodes; the fixed `[..; 8]` generic array becomes a `Vec`-by-id list in
an arena; kinds and stages become two small fields reachable on every term. The
normalizer/unifier are extracted from `typing.rs` into a service that
extensions can extend by registering rewrite rules.

---

### 3.3 The Engine — where proofs live

**What it is.** The single point in the compiler where "does X hold?" questions
are answered. Everything that is currently a hand-rolled lookup or a hardcoded
rule becomes a *query against registered rules*:

- Does `T` satisfy bound `B`? → a **goal**. (Today: nothing; bounds don't exist
  yet — but method lookup via `MethodTable` in
  `crates/hir/src/context/types/methods.rs` is the same shape.)
- Is type `T` `Copy`? → a **goal** with a rule from the builtins/language-core
  extension. (Today: `is_copy_type` calling a hardcoded match in
  `crates/hir/src/ownership/mod.rs`.)
- Which `Move`/`Copy` opcode does this expression lower to? → the codegen
  *queries* the result of the ownership analysis (today it reads
  `LoweringState.ownership` in `crates/codegen/src/lowerers/mod.rs`).

The engine is two solvers over one shared substrate:

1. **Goal engine (top-down, goal-directed).** For existence questions: "prove
   `satisfies(Vec<int>, Iterable)` from these clauses." This is what Chalk does
   for Rust, and the internal mechanism matters:
   - Clauses are Horn clauses: `satisfies(T, Bound) :- Clause1, Clause2, ...`.
   - Resolution is SLD resolution, with a **unifier** (our section 3.2 unifier)
     doing the term-matching.
   - Termination comes from **tabling**: the same goal reaching the same
     unready state is detected and assumed (coinductive/`assume` answers), so
     `impl<T: Clone> Clone for List<T>` does not loop forever.
   - The result of a successful proof can carry *answers* — a substitution of
     the goal's metavars (this is how "what is the type of `Self.Item` for this
     concrete `Self`?" is answered: it is a projection goal with an answer).

2. **Bottom-up engine (fixpoint).** For whole-program, monotone analyses:
   facts are added, rules fire, new facts get added, until nothing changes.
   This is Polonius. Its internal mechanism is a **semi-naive fixpoint**: each
   rule is evaluated only against facts added in the previous iteration
   (incremental), and termination is guaranteed because the fact set is finite
   (bounded lattice, stratified rules). The ownership analysis, the dataflow
   analysis of [`03`](03-typestate.md), and the borrow model of
   [`06`](06-move-semantics.md) are all this shape.

These two are the "Prolog vs Datalog" question from the brief, and the honest
answer is **both, deliberately**:

- *Existence in an infinite/unbounded space* (trait satisfaction over user
  types, "does an impl exist for an arbitrary `T`?") is a **goal** problem —
  the search is over proofs in an unbounded type space; backtracking is fine,
  termination is by tabling.
- *Whole-program finite analysis* (every place in this function, what is its
  state after every block?) is a **fixpoint** problem — the search space is the
  finite product of (place × lattice element), monotone, guaranteed to converge.

They share the same *term* substrate and the same *rule* representation, so an
extension registering a predicate does not choose an engine up front; the
driver chooses: per-query, goal-directed-with-tabling or
bottom-up-with-semi-naive. Chalk ships both personalities in `chalk-engine`
(its `Solver` backend is precisely a bottom-up fixpoint over the goal tree), and
Polonius is Datalog *by construction*, so this "two engines" design is not
novel — it is the convergence of what the two hardest analyses in Rust
eventually used.

**Fact store and query cache.** Every predicate has a store of asserted facts
(which `impl`s exist, which places are moved, which variables a block kills).
All queries are *memoized*. This is the rustc query model / Salsa model: a pure
question asked twice is answered from the cache, and the driver (section 3.5)
owns *invalidation* — when an extension changes the world (a comptime-synthesized
impl appears; a new function becomes visible), the affected query is evicted.
Section 7 of the tensions doc
([`07`](07-tensions.md)) discusses why the driver, not the engine, owns
invalidation and caching: the solver must stay a pure function of its input.

**What already exists to harvest.**

- `MethodTable` (`crates/hir/src/context/types/methods.rs`) — a hand-rolled
  "does this type have this method under this name" lookup. It is the germ of
  the goal store; its answers are facts and its queries are goals.
- `LangItems` (`crates/hir/src/context/lang_items.rs`) — a fixed global
  registry of "the compiler must find exactly this thing": the origin of the
  idea of *core-known, but look-table-resolved* bindings.
- Monomorphization **memoization** already exists in
  `crates/monomorphizer/src/lib.rs` (a `cache` from
  `MonomorphizationKey → specialized declaration`). Cache-aware compilation is
  not new here — it just needs to become universal.

**What must change.** Every hardcoded predicate becomes a registered rule. The
monomorphizer stops being "a pass" and becomes "a query consumer that reads the
driver's worklist" (section 3.5).

---

### 3.4 Flow — the CFG IR and the pluggable dataflow solver

**What it is.** Analyses that depend on *ordering of execution* need a graph,
not a list. Moves, borrows, typestate transitions all answer questions of the
form "by the time execution reaches point *p*, what do we know?" The core
provides:

1. An IR with a **control-flow graph** over **basic blocks** with explicit
   terminators — i.e., the graph the current analyses are missing.
2. A **generic dataflow solver** parameterized by three things an extension
   supplies: a *lattice* (elements + join/meet + top/bottom), *transfer
   functions* (one per instruction class: what a `Call`, a `Move`, an
   `Assign` does to the lattice), and a *direction* (forward/backward). The
   solver iterates to a fixpoint and reports per-point states. This framework
   makes "definite assignment", "use after move", and "typestate invariant"
   all the *same program* with different plug-ins.

**Why this is the second most important primitive.** Today the ownership
analysis runs over **flat HIR statements** (`crates/hir/src/ownership/`), not a
graph:

- It is per-function, in declaration order, a forward walk.
- `PlaceState` is a hardcoded three-field struct
  (`borrowed_mut: u8`, `borrowed_immut: u8`, `moved: bool`) in
  `crates/hir/src/ownership/state.rs`.
- Borrows are tracked but never *ended* — `release_borrow` exists in
  `state.rs` but is never invoked, so a borrow effectively lasts to the end of
  the function. That is a sound over-approximation for a single-pass checker,
  and exactly the kind of simplification a real solver would lift.

This is not a criticism: it is the *best possible answer for where the compiler
stands today*. But typestate (per-place, per-block execution states) and a
precise borrow model are irreducibly graph analyses. Building the CFG and the
solver in the core — once — means every future flow analysis is "a lattice plus
transfer functions," an afternoon's work, not a new subsystem.

**What already exists to harvest.**

- The IR already has basic blocks as labels and terminator-ish instructions
  (`crates/ir/src/model/instruction.rs`, labels in `crates/ir/src/ir.rs`).
- `crates/ir/src/cfg/mod.rs` builds a **petgraph `StableDiGraph<BasicBlock,
  EdgeKind>`** with `EdgeKind::{Unconditional, ConditionalTrue,
  ConditionalFalse, Backedge, Exit}`. The graph construction exists; the solver
  does not — carrying a lattice and running to a fixpoint is missing.
- `crates/hir/src/ownership/place.rs` already turns expressions into
  first-class `HirPlace`s (`Variable`, `Field`, `Index`, `Deref`,
  `Temporary`) — a usable *place* concept to build lattices over.
- `impure_instructions` in `crates/ir/src/ir.rs` — the IR already flags
  side-effecting instructions, which is where order-sensitive analysis cares.

**What must change.** Where the analysis runs. Today ownership runs on *HIR
provirtualization*, before monomorphization. Flow needs concrete kinds of
instruction to be meaningful. The recommended boundary, argued in
[`06`](06-move-semantics.md) section 5, is a **post-specialization CFG IR**:
after the monomorphizer has made types concrete, the backend asks flow
questions. This moves ownership from before specialization to after it — a real
reordering of the pipeline, with a documented compatibility cost for the
currently-on-the-fly codegen tests.

---

### 3.5 The Driver — worklist, queries, amplification, reification

**What it is.** The part of the core that decides *what happens next*. Today
the pipeline is a fixed linear chain owned by `SlynxContext` +
`CompilationStages::build_stages()` in `src/compilation_context/mod.rs`. The
driver has four responsibilities:

1. **Worklist scheduling.** The pipeline becomes a worklist loop with an
   explicit frontier. When a feature says "compiling this triggered the need to
   compile *that*" (a specialised function appears, a comptime block asks for a
   type, a trait impl is generated), it pushes work onto the driver's queue.
   Fixpoint exits when the queue empties. This is the only way comptime
   (section 3 of [`05`](05-comptime.md)) and dynamic trait dispatch synthesis
   (section 4 of [`01`](01-traits-and-associated-types.md)) can be expressed:
   they are not passes; they are *work generators*.

2. **Query cache + invalidation.** The engine's memoized answers (section 3.3)
   are only safe while the world they derived from is unchanged. The driver
   tracks, per query, which extension-provided facts it depended on, and evicts
   on change. This is the salsa contract: **queries are pure; effects of
   imperfection live in the driver.**

3. **Amplification hooks.** Transformation points where extensions attach
   before/after behavior on specific node classes: "after a `FunctionCall` is
   lowered, the effects extension may inject a `sync` instruction." The core
   supplies a stable, position-named set of hooks (the list is in section 5.2)
   and an ordering rule among hooks (the manifest order, section 6.3).

4. **Reification.** The operation that takes a compile-time-known *value or
   term* and splices it back into the program — an emitted literal, a
   synthesized struct, an entire trait-impl record. This is the comptime bridge
   and the trait-dispatch bridge simultaneously. In one sentence: **everything
   that a "macro" or "vtable" or "comptime splice" does in other compilers, we
   do as the same operation: reify an arena value into the IR.**

**What already exists to harvest.**

- `CompilationStages::build_stages()` — the linear driver; the shape that
  becomes the worklist.
- The monomorphizer's `in_progress` cycle detection and `dead_code` set —
  the germ of a frontier.
- Lang item lookup (`crates/hir/src/context/lang_items.rs`) — the germ of "the
  compiler triggers compilation of a specific known binding."

**What must change.** Linearity. `build_hir → monomorphize → build_ir` becomes
`loop { pop work; compile one unit; queries answered from cache; if work,
schedule }`. This is the single most invasive change to the current pipeline,
which is why it appears last in the implementation order doc
([`08`](08-implementation-order.md)) as a *deliberate* final step, not an early
one.

---

## 4. What the Core Explicitly Does NOT Know

It is as important to say what the core refuses to know as what it provides.
The core will ship with **zero** of the following concepts baked in:

- `trait`, `impl`, bound, associated type — the traits doc.
- Kind constructors, type lambdas, `f<_>` — the HKT doc.
- States, automata, state transitions — the typestate doc.
- Effect rows, handlers, capabilities — the effects doc.
- `comptime`, staged evaluation, splicing — the comptime doc.
- `Copy`, `Move`, move semantics, ownership — the move doc. (The *keywords*
  `let`/`let mut` are core; the *semantics* of copying and moving is a
  registered lattice + rules.)

The core *knows* the plumbing these features need: pools, terms, kinds, stages,
unification, goals, rules, fixpoints, CFGs, lattices, hooks, worklists,
caches, reification. It also knows the *shape of features* — that an extension
can introduce new syntactic forms, new declaration kinds, new laws, new
transforms — because that is the protocol of section 6.

There is one exception, admitted openly: **builtins like `int`, `float`, `str`
and the "copy-ness" of primitives come from a *language-essential extension***
(the `std` / `builtins` extension), not from the core, because Slynx's own
intrinsics pattern (`@intrinsic("color")`, documented in
`docs/contributing/intrinsic-types-codegen.md`) already proves that "types the
compiler must know about" can be registered userland bindings. The core stays
feature-free; the builtins extension is simply an extension that always ships
with the language.

---

## 5. The Extension Contract

An extension is a Rust crate — or, when the language matures, a compiled
`.slx`-extension artifact — that does three kinds of things:

1. **Introduces declarations** into the arena (new declaration kinds, new type
   node kinds), through pool slots the driver hands out.
2. **Registers rules** into the engine (goals and/or bottom-up rules) and
   **hooks** into the driver's amplification points.
3. **Requests work** (push onto the worklist), **reads answers** from the
   query cache, and **emits artifacts** through reification.

### 5.1 The manifest

Each extension declares a *manifest* with these fields:

| Field | Meaning | Consumer |
|-------|---------|----------|
| `order` | Where it runs relative to other extensions (a stable total order) | Driver |
| `syntax` | New keyword/token forms it wants reserved/recognized | Parser hooks |
| `declaration_kinds` | New `AnyLocalDeclarationId`-style kinds it owns | Arena layer |
| `type_terms` | New term nodes (each declaring: children arity, kind, name) | Term store |
| `predicates` | New engine predicates, with rule bodies (goals and/or bottom-up) | Engine |
| `lattices` | New dataflow lattices + transfer functions | Flow solver |
| `hooks` | Amplification points it wants to be wired into | Driver |
| `reification` | How its compile-time artifacts splice into the IR | Driver/IR |
| `well_formedness` | Extra legality checks (`trait` must be `impl`ed before use, etc.) | Driver, as queries |

Manifest fields are *registration data*, not code, except for `hooks` and
`lattices`, which are code — functions passed by reference. Rust allows this
kind of registry natively; there is no required dynamic-loading story in the
core. (A future `.slx`-as-extension story — the Racket `#lang dialect` dream —
reuses the same manifest as *data*, because a manifest is just a table. The
core does not need to load foreign code to read a table.)

### 5.2 The amplification slots

The driver exposes a fixed, named set of hook positions. Every hook is named by
*what node class + what phase*, so an extension can say "I want to see every
`FunctionCall` after type checking", declaratively:

```
BEFORE-BUILD:      before a declaration's HIR is built
AFTER-BUILD:       after a declaration's HIR is built, before type checking of children
AFTER-TYPECHECK:   after type checking resolves/caches types for a declaration
BEFORE-LOWER:      before a declaration is lowered to IR
AFTER-LOWER:       after a declaration is lowered to IR
BEFORE-INST:       before an instruction is emitted (per IR instruction class)
AFTER-INST:        after an instruction is emitted
REIFY:             when a compile-time value is spliced into the IR
QUERY-MISS:        when a query returns "no answer" (let extension refine it)
```

Two rules make hooks safe:

1. **Positional stability.** The core never invents new hook positions casually;
   adding one is an event (it is the core's only way to grow, and it must do so
   rarely and deliberately, like a syscall).
2. **Deterministic order.** Hooks at the same position run in manifest `order`.
   This is what makes extensions *compose* without a global registry of every
   pair interaction.

### 5.3 Rules vs. hardcoded behavior — the reified traversal rule

When an extension adds a node kind or declaration kind the core has never seen,
the core must still be able to:

- **walk it** (children of the term tree),
- **render it** for errors,
- **map over it** (substitution, traversal), and
- **cache about it** (hash/equality).

The term protocol provides all four mechanically from two facts: *child ids* and
*kind*. No `match` in preexisting core code needs to gain an arm to host a new
feature. This is the **reified traversal rule**, and it is the entire trick of
keeping a moddable core small: the features add *data*, and preexisting
algorithms operate on *data's shape*, not on feature-specific meaning.

The one unavoidable cost: {meanings the core must not have} are exactly the
{$semantic analyses that must be registered rules, not core {matches}}. Every
feature doc in this series follows that discipline: **feature semantics live in
an extension as rules; the core provides shape, numbers, pools, and loops.**

---

## 6. Worked Example: How the "Generics" Extension Registers

The brief asks for a concrete illustration of "a hypothetical 'generics
extension' registering in this core." Here it is. It is written in illustrative
pseudocode; it is *not* a proposal for final API syntax. The point is to show the
*kinds of slots* an extension fills, using a feature that already exists in the
compiler today.

Recall how generics work today, to appreciate the mapping:

- `crates/hir/src/generics.rs` provides the three shared steps: declaring named
  type parameters, building a `GenericTypeArguments` mapping that fills holes,
  and substituting types through the theory.
- The monomorphizer (`crates/monomorphizer/src/lib.rs`) specializes every
  declaration kind (`functions`, `structs`, `components`, `enums`) with every
  distinct concrete argument list, memoizes the result, detects cycles, and
  drops dead code.
- Two genuinely unsupported cases sit behind `assert_no_generic_non_functions`
  (`unimplemented!()`): generic **type aliases** and generic **stylesheets**.

Now the extension-manifest view (illustrative):

```text
extension "generics" :
  order: 20

  # --- syntactic forms it owns ---
  syntax:
    declare type param   <>        # <T> already parsed by parser; registration makes it meaningful to the core
    generic application  ::= <T, U>   # explicit type-app syntax (current call sites: identity<i32>(x))

  # --- declaration kinds it owns ---
  declaration_kinds:
    generic_alias   ::= data(alias, generic_params, subst_norm)     # today: unimplemented!, owned by this ext
    generic_style   ::= data(style, generic_params)                 # today: unimplemented!, owned by this ext

  # --- term vocabulary it owns ---
  type_terms:
    Apply ::=  arity: many, kind: built from (kind of head) − 1 per arg   # <f, args...>
    GenericParam ::= arity: 0, kind: Type                                  # a named, well-formed type variable
    GenericComponentRef ::= arity: 1, kind: Type                           # today: HirType::GenericComponent

  # --- laws (engine rules) ---
  predicates:
    substitution law find(arg-lits) in every `Apply` of a generic decl.
      (compiled from the three steps in crates/hir/src/generics.rs)
    specialize(Declaration, ArgList) --> SpecializedDeclaration
      (the monomorphizer's cache becomes a *query*: memoized in the engine)

  # --- well-formedness ---
  well_formedness:
    generic_alias / generic_style must be instantiated before reference.
      ("assert_no_generic_non_functions" becomes a query the aliases/styles ext answers)

  # --- work & reification ---
  reification:
    a SpecializedDeclaration from the `specialize` query reifies as an
    ordinary declaration record in the arena (same `AnyDeclarationId` slot),
    which downstream passes consume exactly like source-declared ones.
```

The profound part is not the generics-specific content; it is that every one of
these slots is *generic*. The traits extension fills the same slots with:
declaration kinds `trait`/`impl`, term nodes `trait_ref`/`projection`, a
`satisfies(T, Bound)` predicate with SLD clauses, and a well-formedness law for
coherence. The typestate extension fills them with `typestate` declaration
records, per-place lattice terms, and transfer functions instead of rules. The
*driver* does not care which extension filled the slots: each slot has the same
contract, and the manifest `order` resolves inter-extension scheduling.

---

## 7. Ordering Rules Among Extensions

Because the core rejects a global cross-product registry ("trait × effects ×
typestate interactions" is not a table the core ships), it imposes three
ordering contracts that make those interactions *composed* rather than special:

1. **Analysis feeds forward by lattice.** Two flow extensions do not "talk";
   the earlier one's *result liveness* (e.g. "this place cannot be `move`d
   after the transfer") feeds the later's *transfer functions* as a query. The
   move extension's lattice is the lowest layer; typestate builds on it. The
   tensions doc ([`07`](07-tensions.md)) section 4 pins the exact contractual
   dependency.

2. **Resolution is additive.** Any extension may add rules; no extension may at
   compile time *delete* a rule another extension registered. Revisability
   exists only at the *driver* level (invalidation), and only for
   worklist-scheduled re-resolution — never silently in the middle of a query.

3. **Well-formedness is a query, not a pass.** Every legality check an
   extension installs ("a `Socket` in state `Closed` may not be sent through" /
   "an unhandled effect at the top level") is a *question*, memoized by the
   engine, subject to invalidation like everything else. This is what keeps the
   driver a uniform loop: correctness checks are just queries with side effects
   (diagnostics).

---

## 8. Risks and Open Design Questions

**Risk 1 — The core is five subsystems, which is not "small."** The core is
*small in concept surface* (zero features baked in), not *small in subsystems*.
The fix is the game-engine framing: subsystems are cheap and shared; feature
meaning is the expensive, hardcoded part we are eliminating. A fair reviewer
will still measure us by SLOC of `*core*`, and the honest target is: every
mechanism must serve at least one existing feature *today* (terms serve
generics; flow serves ownership; the driver serves monomorphization). Nothing
is speculative.

**Risk 2 — Genericity taxes.**
* The unifier over extensible terms is slower than today's closed-enum
  `unify_types`. Mitigation: the dedup pools already intern aggressively; cost
  is amortized.
* Query-cached resolution must stay *pure*. The moment a hook mutates the world
  mid-query, cache soundness dies. The driver owns mutation; the engine forbids
  it. This rule needs to be *enforced*, not just written down.

**Risk 3 — The CFG/flow reordering.** Moving ownership from pre- to
post-specialization changes when errors are reported and which tests exercise
them (`tests/move_semantics.rs`, `tests/ownership.rs`). This is the most
user-visible ripple of the whole plan; [`06`](06-move-semantics.md) section 5
and [`08`](08-implementation-order.md) section 5 schedule it explicitly and
isolate its blast radius.

**Open question — dynamic loading.** The manifest-as-data design deliberately
supports a future `.slx` extension story without requiring it now. Whether to
pursue it (and whether a plugin ABI is needed) is explicitly out of scope of
this series; the docs assume extension = Rust crate, with the manifest as the
registration point.

**Open question — the exception to "no features in core."** Builtin types,
`@builtin` lang items, and the `Style`/`Component` notions smuggled through
`@intrinsic` today. The position taken here (and defended in
[`04`](04-effects.md) and the overview) is: these are the *builtins extension*,
not core content. The core therefore has exactly one non-feature concept of
"the language": *zero*. Even `let`/`let mut`, `func`, `object` can later be
described as the syntax the builtins extension owns.

---

## 9. Reconciliation with the Existing Pipeline

This doc must give a fair account of *what survives*. The redesign is
invasive but not destructive — nearly every subsystem is *harvested*:

| Today | Becomes |
|-------|---------|
| `HirStore`, `TypeStorage`, `SlynxIR` free lists | the **Arena** (3.1), with a pool factory |
| `HirType` closed enum + `substitute_types` | the **Terms** (3.2) tree, unification service |
| `MethodTable`, `LangItems`, monomorphizer memo cache | **Engine** (3.3) fact store and query cache |
| `crates/ir/src/cfg` + `PlaceState`/`ExpressionUse` | **Flow** (3.4) solver with two registered lattices |
| `CompilationStages::build_stages()` linear chain | **Driver** (3.5) worklist + invalidation |
| `HirAttributeKind::Capabilities` orphaned stub | an **effects extension** manifest entry (section 3 of [`04`](04-effects.md)) |

The next document in the series
([`01-traits-and-associated-types.md`](01-traits-and-associated-types.md))
takes the first feature and walks these primitives end to end. The reader who
understands this overview will find the rest of the series predictable: each
feature doc asks the same five questions —

1. What exists in the codebase today?
2. What must the core provide (in terms of primitives)?
3. What rules, lattices, hooks, and terms does the extension register?
4. What did we consider instead, and why did we refuse it?
5. What does the compatibility fallout look like?