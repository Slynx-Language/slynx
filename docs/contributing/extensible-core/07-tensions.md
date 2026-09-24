# Tensions: What Conflicts with What, and How the Core Refuses to Choose

> Series: *extensible-core*. Assumes [`00-overview.md`](00-overview.md). A
> *synthesis* document: the six feature docs each describe a feature in
> isolation; this one names the places where two features, naively implemented,
> would fight — and the single core principle that resolves each fight without
> the core ever picking a winner.

---

## 0. Why This Document Exists

Every real moddable-core attempt dies on a tension no one listed first: HKT
assumes two-level types while the checker was built for one; comptime wants to
*reamend* the world while trait resolution assumed it was closed; typestate
wants in-place re-typing while move analysis forbids aliases; effects want
creation-site instrumentation while lowering was a fixed walk. The overview
promised (section 7) that the core resolves these by *composition contracts*,
not by choosing. This document is the ledger of those contracts, in one place,
each entry pointing at the doc that farms the mechanics.

Each entry has the same shape: **the naive conflict, the core principle that
dissolves it, and the exact slot where the resolution lives.**

---

## 1. The Meta-Tension: Closed Representation vs. Extensible Universe

**The naive conflict.** Every current representation is a closed enum — `HirType`
(`crates/hir/src/model/types.rs`), `HirExpressionKind`
(`crates/hir/src/model/expression.rs`), `AnyLocalDeclarationId`
(`crates/hir/src/id.rs`), `Opcode` (`crates/ir/src/model/instruction.rs`),
`IRType` (`crates/ir/src/types/irtype.rs`). Making the universe extensible is
*exactly* "delete these enums" — and that is a big, risky, featureless deletion.

**The core principle.** The reified traversal rule (overview 5.3): *shape is
enough*. A pass written before `Trait` existed can still walk a `Trait` term,
render it, substitute around it, cache about it — because every term node
carries *children* and a *kind*, and the core's generic algorithms
(unification, substitution, rendering, hashing) operate on shape, never on
meaning.

**Where it lives.** The term protocol + `terms` primitive (overview 3.2); the
HKT doc's "kinds are measurement" (02 section 3). The concrete cost that makes
this *true, not theoretical*: the fixed `[..;8]` generic array and every
closed-enum `match` in preexisting passes gets replaced by a *shape* match
(child count + kind), with a graceful `QUERY-MISS`/unknown fallback.

**The price, honestly stated.** A pre-extension pass cannot *decide* about a
term it has never seen (e.g. hand it to the old monomorphizer as if it were a
concrete type). It can only *carry* it. "Carry, don't decide" is the contract
every refactored pass commits to; deciding belongs to extensions, via rules.

---

## 2. Single-Kind Flatness vs. Higher-Kinded Types

**The naive conflict.** The current type system has no kinds (HKT doc section
1); everything is assumed kind `*`, and application is "the `[..;8]` list."
HKT wants `f<a>` where `f` has kind `* -> *`, plus lambdas and constructor
bounds. If kinds were a *post-hoc feature*, the checker would have to
re-run every unification with a second level after the fact.

**The resolution.** Kinds are *structural measurement* — a closed, tiny
vocabulary (`Type`, `Fn(...)`, `Universe`, plus inference `Var`) carried on
every term from day one (overview 3.2, HKT doc section 3). `f<a>` is then not
"new syntax the checker must learn"; it is the same application node with a
computed kind. The *user-facing* syntax (`f<_>`, type lambdas, quantifiers)
remains an extension (HKT doc section 4); the *measurement* is core.

**The refusal.** The core refuses `kind` *polymorphism* (a kind that ranges
over kinds). Closed kinds keep the kind level decidable, which every flow
analysis and every comptime loop needs. That refusal is a *price paid now*, a
*future option later*, and never a hidden fork: a language without HKT *still*
runs on exactly the same term+kinds representation (all terms just happen to
resolve to kind `*`).

**Cross-ref.** [`02-higher-kinded-types.md`](02-higher-kinded-types.md)
sections 3 and 7.

---

## 3. Closed-World Static Resolution vs. Comptime's Open World

**The naive conflict.** Trait resolution (`01` section 4) and the monomorphizer
memoize answers under the assumption that the program is *fixed and finite*
when resolution runs. Comptime (`05`) *adds* facts mid-compile — a comptime
block synthesizes an `impl`, a type, a specialized function — which invalidates
every "already resolved" answer that depended on the missing fact.

**The resolution.** That is *the salsa contract* (overview 3.3): **queries are
pure; the driver owns the cache and its invalidation; the engine never mutates
and never re-resolves in the middle of a query.** Comptime adds facts via
*reified work items*; the driver evicts the queries that depended on the
changed inputs, and the *next* request re-resolves. Nothing on either side
changes its protocol: the trait ext just sees "a query came back empty, now it
comes back full," and comptime just sees "I added a fact; the driver promises
the world is consistent again."

**The refusal.** The core refuses to make the *world* a first-class mutable
object shared between solver and comptime. Mutation is *driver-owned and
scheduled*; the solver stays a pure function of its table. That is the
difference between a re-runnable design and a recalcitrant one.

**The real cost.** The linear pipeline (`CompilationStages::build_stages()`)
*dies* — replaced by the worklist fixpoint (comptime doc section 4.2). This is
the single biggest structural change in the series, which is why the impl-order
doc ([`08`](08-implementation-order.md) section 6) schedules it *last*: build
every other extension on the linear pipeline first, observe the loops they
need, *then* convert the driver. The conversion's risk is contained precisely
because by then the queries are already pure and cached.

**Cross-ref.** [`05-comptime.md`](05-comptime.md) section 4.2; overview 3.3 and
3.5.

---

## 4. Static Dispatch vs. Dynamic Dispatch (traits and effects' shared battleground)

**The naive conflict.** Rust-style static dispatch (monomorphized, zero-cost)
and dynamic dispatch (`&dyn Trait`, effect handlers chosen at runtime) are
usually seen as *competing whole-system strategies*: pick one, and the other
becomes a major project (or an optional feature with its own runtime).

**The core principle.** *Neither is core, and the IR stays dumb.* Static
dispatch is the existing monomorphizer answering `specialize` queries. Dynamic
dispatch (traits doc section 4.4, effects doc section 7) is *the same thing
plus a synthesized table*: a per-impl vtable is **reified data** (a struct of
function refs), a fat pointer is **just a struct of one pointer + one ref**, and
a `dyn` handler is **just a `handler_ref` argument**. The core contributes the
*mechanisms* (reification, specialization, the worklist) and refuses the
*strategy*.

**The refusal becomes a feature.** Because the core has no dispatcher, the
"static vs dynamic" decision is a *per-call-site policy of the extension* —
the exact nuance the effects brief asked for (zero-cost when statically known,
opt-in indirection when not). There is no language-wide vote; there is a rule
per call site, and the rule itself can be a query (`provably_static` from the
effects doc), so future refinements ("always dynamic for this trait") are
*another rule*, not an RFC to the compiler core.

**The boundary the core upholds:** nothing new in the IR. Vtables, effect
evidence, handler refs, comptime splices — all are *data* the extension
reifies into ordinary types and arguments; `Opcode` and `IRType` stay closed
and dumb. Any proposal that asks for a new IR instruction for a *feature* is
rejected at the porch.

**Cross-ref.** [`01-traits-and-associated-types.md`](01-traits-and-associated-types.md)
section 4.4; [`04-effects.md`](04-effects.md) sections 4.2 and 7.

---

## 5. Typestate's In-Place Transitions vs. Move Analysis's No-Alias Rule

**The naive conflict.** Typestate re-types a value *in place*; move analysis
says a borrowed place must not change underneath a live reference. Serve both
naively and either (a) every transition is rejected inside any borrow scope —
useless — or (b) transitions bypass the move lattice — unsound.

**The resolution, pinned as a contract.** *Transitionability is a predicate,
answered by the move analysis* (typestate doc section 6, move doc section 4):
`transitionable(place, point)` iff the place is `Live` with zero live borrows
at that point. Typestate's transfer function *queries* it. No borrow ever
crosses a transition; no borrow-free transition is ever rejected.

**Why this composes, not conflicts:** both are separate lattices inside the
*same* flow solver, run in a *fixed order* (move first, typestate second — the
"analysis feeds forward by lattice" rule, overview 7.1). There is no
cross-extension interaction table, because the interaction *is* the query.
Strengthening the alias model later (a whole-program alias-accuracy extension,
move doc section 6) strengthens `transitionable` without touching typestate.

**The refusal with teeth:** the core refuses to let typestate's lattice
*overwrite* move's answers (no "reset all borrows, trust me"). Ordering +
querying is the entire contract; violating it is the one way to make this pair
unsound.

**Cross-ref.** [`03-typestate.md`](03-typestate.md) section 6;
[`06-move-semantics.md`](06-move-semantics.md) section 4.

---

## 6. Bounded Generics vs. the Current `unimplemented` Walls

**The naive conflict.** The monomorphizer's `assert_no_generic_non_functions`
(`crates/monomorphizer/src/lib.rs`) throws on generic **type aliases** and
generic **stylesheets** with no way to say `where T: Bound` about them. Traits
(doc 01) makes bounds exist; if aliases/stylesheets still hit `unimplemented`,
the language is half-open — bounds exist but only inside functions.

**The resolution (and the honest root cause).** Both limits are *symptoms of
one disease*: the compiler assumes a generic declaration can only appear in a
simple call context. The fix is the same reification that comptime needs (doc
05 section 4): an alias/style is a *declaration whose generic parameters are
kinded terms*; instantiating it is `specialize` + `satisfies` (the same query
path as a function). Once the alias/stylesheet generic parameters are
*first-class kinded terms* (HKT doc section 3), both `unimplemented` walls are
naturally lifted — and the `docs/contributing/generics-implementation.md`
known-limits table stops being a scary wall and becomes a *resolved note*.

**The refusal:** the core refuses to special-case `styles.css` or `alias` in
its own logic; the styles/alias story is the `terms`/`engine` story, with the
styles' `apply` desugar intact (docs `styles.md`, `boostraping-components.md`).

---

## 7. Goal-Search (Existence) vs. Bottom-Up Fixpoint (Whole-Program)

**The naive conflict.** One "engine" that must be Prolog-style for `satisfies`
over an *unbounded* type space, and Datalog-style for Polonius-style analyses
over a *finite* lattice — people argue these are different philosophies.

**The resolution (overview 3.3, made precise here):** they are two drivers over
**one substrate** — the same term store, the same rule registry, the same fact
table — because both are, at bottom, "closure over registered Horn clauses with
a finite or tabled termination strategy." Chalk's own `Solver` is a bottom-up
fixpoint over a goal tree; Polonius is a bottom-up engine *by construction*.
The *choice* is per-query: existence questions go goal-directed (SLD + tabling
+ coinductive assumption for `Clone`-style self-referential impls); monotone
analyses go bottom-up (semi-naive, guaranteed finite). The decision is a driver
heuristic keyed on *predicate registration metadata* (an extension declares
whether its predicate is *stratifiable/bounded*), never a recomputation of the
engine's religion.

**The refusal:** the core refuses to collapse the two into "just Prolog" (the
finite-world analyses would lose their termination proof) *or* "just Datalog"
(the unbounded goal space would never close). It also refuses to let
extensions mix arbitrary recursive rules into the bottom-up fragment in a
non-stratified way — the strate is checked at registration, not at runtime.

**Cross-ref.** overview 3.3; the bottom-up facts of [`01`](01-traits-and-associated-types.md)
(coherence) and [`06`](06-move-semantics.md) (Polonius-style loans) are the two
recognizable users.

---

## 8. Effects on Data vs. Comptime's Type-Computation

**The naive conflict.** An effect row on a *value's type* (effects doc section
4.1) is enforced by annotations at creation/use/call points. Comptime computes
*new types mid-compile*. If a comptime block produces a `GpuBuffer`-typed value,
are its capability terms part of the *computed* type — and do the creation/use
checks survive the computation?

**The resolution:** they *are* terms, and terms are *staged* (overview 3.2,
comptime doc section 4). A capability attached to a comptime-computed type is a
*CompileTime-staged capability*; the enforcement goals reuse the same
`ambient_capability` rules, and `REIFY` erases the evidence exactly as it would
for a source-level type. The interaction is *ordinary*, because neither feature
is a compiler law — both are term/rule registrations, and stage is a field on
fractions of the same tree.

**The refusal:** the core refuses a *separate "effect world"* or a *separate
"comptime world"* — two universes where the same type means different things
depending on which feature is talking. One staged term tree; one engine; one
driver.

---

## 9. The One Principle Under All of These

Every resolution in this document is the same move:

> **The core provides shape, order, and purity. Extensions provide meaning,
> strategy, and mutation-of-the-world. Interactions between extensions are
> queries and ordering contracts — never special-cases in the core.**

That is the entire thesis of the movable core. When a tension looks like "two
features cannot coexist," look for the *property* each one truly needs — a
measurement (kind), a purity (query), a predicate (transitionable), a stage
(term) — and give that property to the core as *shape*; the features stop
competing because they now read values instead of defining them.

---

Next: [`08-implementation-order.md`](08-implementation-order.md) — the plan that
turns this architecture into a sequence of low-risk landings, each building on
the last.