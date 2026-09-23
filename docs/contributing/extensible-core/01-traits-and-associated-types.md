# Traits, Bounds, and Associated Types as an Extension

> Series: *extensible-core*. Reading order: this document assumes
> [`00-overview.md`](00-overview.md). It walks the first full feature through the
> five core primitives (arena, terms, engine, flow, driver) and answers the five
> standard questions: *what exists today, what the core must provide, what the
> extension registers, what alternatives we refused and why, and what the
> compatibility fallout is.*

---

## 1. The Feature, Defined By Its Problem

A trait system answers two questions that the language cannot currently ask:

1. **"Does this concrete type provide this capability?"** — Slynx today has
   *methods*, but a method is owned by exactly one type (`MethodTable` in
   `crates/hir/src/context/types/methods.rs` maps a type id to its methods).
   There is no *interface* a type satisfies, so a function cannot say "give me
   any `T` that can `format` itself."
2. **"What must the compiler *assume* about an unknown type in a generic
   function?"** — generics exist and monomorphize
   (`crates/monomorphizer/src/lib.rs`), but a generic parameter has *no
   bound*: `identity<T>(x)` cannot call `x.fmt()` on `x`.

The feature under discussion is the classic OO/"type-class" package, delivered
as an extension:

- **Traits** (aka interfaces / type classes): a named set of method signatures a
  type may promise to provide.
- **Bounds**: `where T: Display` — the assumption a generic function may place on
  its parameters, and the obligation the caller must discharge.
- **Associated types**: `Self::Item` — a trait may say "whatever `Self` is,
  there is exactly one type I call `Item`, and it must satisfy `: Clone`."

The brief demands three things of this doc:

- What the *mechanism* is in reference languages (with their real internals), so
  we can steal the good parts.
- What exists in the Slynx codebase today, so the extension is welded onto real
  seams, not imagined ones.
- What the *core must provide* versus what the *extension registers*, per the
  overview's protocol.

Section 2 is the mechanism; section 3 is the codebase today; sections 4–6 are
the design; sections 7–8 are alternatives and fallout.

---

## 2. How Real Systems Implement This (the mechanism, from the inside)

### 2.1 Rust: two dispatch strategies sharing one idea

Rust's trait system has two execution strategies, and every serious design for
traits in Slynx must decide which one to build on — or that it builds both.

**Static dispatch (generics + monomorphization).** Generic functions are
*instantiated* per concrete argument type. The mechanism that makes methods
findable is **dictionary passing**: `fn print<T: Display>(x: T)` compiles as if
it took a hidden extra parameter — a reference to a `Display` impl record (the
"dictionary" / vtable) — plus the value `x`. Because each monomorphized copy is
for one concrete `T`, the dictionary reference is *statically known*, so LLVM
inlines the method call into its body. Zero indirection cost. This is exactly
the machine Slynx already has: the monomorphizer
(`crates/monomorphizer/src/lib.rs`) already produces one specialized copy per
concrete argument list; notably for its *current feature set*, *everything is
generic or concrete, nothing is dispatch-by-pointer.*

**Dynamic dispatch (`dyn Trait`).** `dyn Display` is a **fat pointer**: two
words — a data pointer to the value, and a pointer to the `Display` vtable
(a per-concrete-type table of `fn` pointers plus size/alignment/drop
metadata). Method calls go through the vtable: one indirect jump. Rust
enforces **object safety** as a well-formedness rule — you cannot make `dyn
Sizedmethod` (methods generic over their own `Self`) into a trait object,
because the vtable would need an infinite set of entries. The key internal
fact: **the vtable is not a runtime concept in Rust's front end; it is `&'static`
data synthesized by the compiler** (a `&dyn` value is literally a reference to
a per-impl struct).

### 2.2 Chalk: trait solving as logic programming

Chalk is the re-implementation of Rust's trait solving as a logic engine, and
it is the best documented real system for the "engine" primitive. The internals
that matter:

- Every `impl` is *lowered* into **Horn clauses**:
  `satisfies(T, Display), T: From<U> → satisfies(U, Display)` — clauses over a
  small term language where `satisfies(T, Bound)` is just a predicate symbol.
- **SLD resolution** (goal-directed, depth-first, like Prolog) proves goals by
  repeatedly unifying the head goal's term against clause heads.
- **Termination by tabling.** Recursion between interdependent impls
  (`impl<T: Clone> Clone for List<T>` plus `impl Clone for int`) could loop
  forever. Chalk keeps a *table of in-progress goals*: when a goal re-encounters
  a term it is already busy proving, the cycle is assumed to hold (for the
  "coinductive" fragment — well-founded derivations), and search proceeds on the
  remaining subgoals. Every solver worth copying does this.
- **Answers.** A proof can produce *answers*: metavariables in the goal are
  solved by the resulting substitution. This is how associated-type projection
  works in Chalk — `Normalize(<T as Iterator>::Item -> ?A)` is a goal whose
  successful proof *is* the answer `?A = int`.
- **Coherence is NOT the logic.** Whether two impls overlap (making dispatch
  ambiguous) and whether an impl is "orphaned" (defined where neither the trait
  nor the type lives) are **separate well-formedness checks** outside the solver.
  This separation is central to our design: the *engine* proves what's true; the
  *extension* decides what a legal program is.

### 2.3 Associated types

An associated type is a trait-level type-level function from `Self` to `Type`.
Its mechanism is **normalization**: `<Vec<int> as Collection>::Item` rewrites
to `int` by finding the (unique by coherence) impl of `Collection for
Vec<int>` and substituting into its `type Item = ...`. Two design consequences
are baked into Rust and worth preserving:

- **Single result.** A trait can have at most one associated value per (Self,
  type-params); therefore normalization is a *function*, not a relation, and
  the engine can be optimistic about it.
- **Generic traits vs associated types** are two halves of one coin. A *generic
  trait* (`trait Foo<T>`) gives *multi-parameter dispatch* — the impl is chosen
  by two types — which is more expressive and more ambiguous. An *associated
  type* gives *single-parameter* dispatch with an output, which is cleaner and
  always resolvable. Rust has both and the community oscillates between them;
  the honest guidance from history (the `std::ops::Add<T>`-vs-`: Iterator`
  distinction) is: **use generic traits when the "extra parameters" are genuinely
  inputs to the interface (a `Container<T>` you can *add anything into*), use
  associated types when they are outputs derived from `Self`.**

---

## 3. What Slynx Already Has (and is ripe for harvesting)

There is no `trait` today. There is a great deal of *shape* that is one reshape
from a trait system:

| Existing mechanism | Where | What it already does for us |
|--------------------|-------|-----------------------------|
| `MethodTable` | `crates/hir/src/context/types/methods.rs` | Type→methods lookup. This is **inherent-method dispatch**, i.e. the traits extension's *fallback case* (a type's own methods are just the "impl of nothing an extension attaches"). |
| `LangItems` | `crates/hir/src/context/lang_items.rs` | A global name→decl registry the compiler consults for special bindings. This is the germ of "the compiler must resolve *this specific named entity*": a trait system's `Clone`/`Display` are lang-item-adjacent. |
| `@builtin` attribute | `crates/hir/src/builders/attributes/mod.rs`, `model/declarations.rs` | Registration from source into a compiler-known registry. The mechanism a "builtin trait" would use. |
| `intrinsic` objects | `docs/contributing/intrinsic-types-codegen.md` | Proves "types the compiler must know are userland-registered" already works. |
| Generic machinery | `crates/hir/src/generics.rs`, `docs/contributing/generics-implementation.md` | The `GenericTypeArguments` + `substitute_types` steps (declare params, build holes, substitute) generalize directly to trait parameters: a trait bound is just a *constraint term that gets substituted*. |
| Mono cache | `crates/monomorphizer/src/lib.rs` | Memoized specialization; becomes the default answer storage for "which concrete impl did we pick." |
| `Style` desugar | `docs/contributing/styles.md`, `docs/contributing/boostraping-components.md` | Stylesheets are already *lowered into an interface plus an `apply` function* — a conceptual precedent that "portable capability surfaces" exist in the language model. |
| `concept` keyword | mentioned in `docs/contributing/generics-implementation.md` (reserved, unsupported) | The grammar has already staked out vocabulary; no implementation exists. |

Two *hard limits* the extension must lift are visible today:

1. **Generics cannot be bounded.** A generic param is either the builtin
   "anything at all" or explicit concrete arguments; there is no middle
   solution space ("constructed, but subject to constraints"). Concretely:
   generic functions can call *no* methods on their parameters.
2. **`assert_no_generic_non_functions`** (`crates/monomorphizer/src/lib.rs`)
   throws `unimplemented!()` on generic type aliases and generic stylesheets.
   Bounds+trait support and alias support are siblings: both need nested
   generic contexts to mean something.

---

## 4. The Design: Traits as Declarations + Terms + Rules

### 4.1 What the core must provide

Nothing new beyond the five primitives of the overview; that is the point of
this exercise. Concretely the core contributes:

- **Arena** — pool slots for `trait` and `impl` declaration records, and for
  method-id→decl maps, handed out via the driver's pool factory.
- **Terms** — the extensible term tree with kind + stage, the unifier, and the
  **normalizer with pluggable rewrite rules**. The projection rewrite is a
  registered rule ("rewrite `<T as Trait>::Assoc` by querying the impl store"),
  not a core match arm.
- **Engine** — `satisfies(T, Bound)` as a *registered predicate* with clauses
  supplied by this extension; memoization + tabling for termination; a query
  cache the driver can invalidate.
- **Driver** — a reification hook so the extension can *synthesize* impl
  records/vtables and splice them into the arena as ordinary declarations, and
  `well_formedness` query slots for coherence.

### 4.2 What the extension registers (the manifest view)

```text
extension "traits":
  order: 30

  declaration_kinds:
    trait  ::= data(name, generic_params, supertrait_terms, method_model)
    impl   ::= data(trait_ref, self_ty, generic_params, method_bodies, assoc_ty_bindings)

  type_terms:
    trait_ref     ::= arity: N  (N >= 1)   kind Type   # Trait<T...>, Self = first arg
    projection    ::= arity: 2  kind Type             # <T as Trait>::Assoc
    bound_holder  ::= arity: 2  kind Type             # T : Bound (a constraint that must resolve)

  predicates (goal form, SLD with tabling):
    satisfies(SelfTy, TraitRef, Params<...>)   :-  Clause per impl in scope.
    method(TraitRef, MethodName)  -> MethodDecl        # lookup, = today's MethodTable as a query
    normalize(Projection)         -> ConcreteType      # via the unique impl

  bottom-up facts (for coherence answers only):
    overlapping(impl_a, impl_b)   :- deduction over self-type generalization

  well_formedness (queries, memoized):
    coherence:  no two live impls of the same trait for the same self-type
    orphan:     an impl must live where at least one of {trait, self-type root} lives
    object_safety: static vs dynamic strategy can be chosen (section 4.4)

  reification:
    an impl record -> an ordinary arena declaration (downstream sees source-like decls)
    a vtable       -> a struct of method pointers + size/align/drop slots, synthesized data
```

The *only* new engine semantics here — and it is not new, it is Chalk's — is "a
clause head may be a predicate call over terms." Everything else (unification,
substitution, memoization, invalidation) is core plumbing.

### 4.3 Bounds and where they get checked

A bound `T: Display` lowers to a *nesting obligation*: the generic function
body carries a `bound_holder` term for each `where` constraint, and each call
site must discharge it with a *proof* — a `satisfies(ConcreteTy, Display)`
goal whose clause set includes the user's impls *plus the current generic
context's assumed bounds*. This is exactly Chalk: the generic parameters
"in scope" contribute their assumed-reaches predicates to the clause set while
solving inside the function body. The monomorphizer doesn't need to change its
core logic: the obligation is discharged *at instantiation time* in the engine,
and `specialize(Declaration, ArgList)` (the overview's worked example) becomes
a query precondition — `satisfies(ConcreteTyA, BoundOf<ParamB>)` must hold
before specialization is committed.

### 4.4 Static vs dynamic dispatch: choosing in the extension

Both strategies are *strategies of one extension*, not two features:

- **Static** — exactly the existing monomorphizer path. The "dictionary" is
  folded into the specialized body; zero runtime cost. Default.
- **Dynamic** — the extension asks the *driver* to synthesize, at
  compile-time, a per-impl **vtable struct** (data pointers + fn pointers)
  using the *comptime/reification* machinery — **not a new IR primitive.** A
  fat pointer is two fields: `value: *mut u8`, `vtable: *DynamicTracker` — and
  in Slynx a *pointer + one object* is already expressible as an `object` with
  the ref types from `crates/ir/src/types/irtype.rs`. The payoff of this design
  is huge for core-size: **the core has no "vtable" concept**, because a vtable
  is just *data* (a struct of function references) and a fat pointer is just
  *a struct*. Rust's `&dyn Trait` is the same two words; it *appears* special
  because the compiler synthesizes the vtable — which is reification.

The brief's *real* question — "static monomorphized trait resolution vs
runtime dispatch" — is answered in the tensions doc
([`07`](07-tensions.md) section 5): it is *not* a core decision, it is a
per-call-site policy of the traits extension, and the core enables both by
offering reification + specialization + a worklist.

### 4.5 Associated types and normalization

`type Item` binds in the `impl`; the projection term `<T as Trait>::Item`
normalizes through the unique impl — *unique* because coherence is a
well-formedness query. The engine's normalizer applies the registered rewrite
rule; when it needs the impl, it issues the `method`/impl-lookup goal and the
query cache answers. The only subtlety — and it belongs in the extension, not
the core — is that normalization can be *delayed*, leaving a projection term
in a type until the self-type is known (this is what Rust calls "keeping
`T: ?Sized` reasonable"). The core's terms handle it because a `projection` is
an ordinary term node: it can sit in a type, be substituted, be rendered, and
later *not normalize* — the normalizer just applies rules to whatever terms are
in scope.

---

## 5. The Example: `Collection` with an associated element

Illustrative Slynx syntax (not final; the parser reserves no trait keywords
yet — `concept` is reserved, `interface` is used only in style lowering).

```slynx
trait Collection {
    type Item;
    func len(&self) -> int;
    func at(&self, i: int) -> Self.Item;       // an associated-type projection
}

/*
   Hypothetical performance note, for the reader who checks the IR:
   at(&self, i) returns Self.Item, i.e. a projection node in the type tree.
   Inside generic code it stays a projection term; after the monomorphizer
   picks Vec<int>, the projection rewrites to int and the call folds into
   the vec's specialized len/at. That is the entire cost story: static when
   provable, synthesized-table otherwise.
*/

impl Collection for Vec<int> {
    type Item = int;
    func len(&self) -> int { ... }
    func at(&self, i: int) -> int { ... }
}

func first_and_show<T>(xs: T) -> str where T: Collection, T.Item: Display {
    let first = xs.at(0);            // type of «first» is T.Item (a projection term)
    return first.to_string();        // discharged by the T.Item: Display bound
}
```

What the compiler *does* with this, mechanically:

1. Parsing + HIR build create `trait`/`impl` declarations in new arena pools.
2. The `where` clause lowers to `bound_holder(T, Collection)` and
   `bound_holder(T.Item, Display)` terms.
3. Inside the body, `xs.at(0)` produces a `method(Collection, at)` goal; the
   engine resolves it to `at`'s signature, whose return type is the *projection*
   `T.Item`.
4. At `first_and_show<Vec<int>>`, the driver pushes a specialize work item. The
   monomorphizer's query fires the preconditions: `satisfies(Vec<int>,
   Collection)` (yes, from the impl) and `satisfies(int, Display)` (yes, from
   the builtins extension) → the body is specialized with `T := Vec<int>`,
   `Int := int`'s impl **cached**, and no further obligation remains.

All of steps 2–4 are queries against registered rules; nothing in the core
matched the word "Collection".

---

## 6. Are Two Traits Different from One? (supertraits and defaults)

Rust's `trait A: B` is a *syntactic sugar for an implied bound*: `A: B`
contributes `satisfies(Self, B)` to the clause set while solving for any impl
of `A` — and to the well-formedness rule "to implement `A` you must implement
`B`." Default method bodies are *also* a sugar — a method stored on the trait
whose record the impl inherits-by-name until overridden. Both are cheap
manifest entries (one clause, one method-inheritance rule); neither needs a
core change. The doc notes them so the reader does not think a "real" trait
system needs anything bigger.

---

## 7. Alternatives Considered and Refused

1. **Structural interfaces only (no nominal impls).** A type "satisfies" an
   interface if it has the right members, checked structurally. Refused: Slynx
   methods are declared per-type and the language has objects with nominal
   identity (`docs/language-surface.md`); structural-only typing fights the
   existing mental model, defeats coherence-driven specialization, and makes
   the "two types both named `open`" ambiguity a runtime/compile-time
   ambiguity. It also gives up the dict/vtable synthesis story for free: with
   nominal impls, the extension can *find* the impl; structurally it must
   *search* for a witness.
2. **Only static dispatch (no trait objects).** Refused despite the monomorphizer
   being perfectly suited: `Vec<dyn Renderable>` and plugin tables require
   *runtime* heterogeneity that one big `match` over all concrete types cannot
   express. The synthesize-vtable path (4.4) costs almost nothing and covers
   both.
3. **Embedding a full Chalk-like solver in the core, with trait solving as a
   core feature.** Refused twice over: it re-introduces a feature into the
   core (the overview's section 4 forbids it), and it drags in Rust-specific
   semantics (orphan rules, object safety) that Slynx should *decide*, not
   inherit. The core keeps the *rule substrate*; the traits extension keeps
   the *rules*.
4. **Associated types only, no generic traits (single-parameter dispatch
   everywhere).** Refused: the language should not force users to wrap
   "extra inputs" in awkward types (the Rust `Add`-trait experience). Generic
   traits and associated types ship together, with the history-derived
   guidance of 2.3.
5. **Interface via '*'/"any object" / Go-style.** Refused for the reason every
   Go programmer eventually meets: no way to express "T has an `at(i)` that
   returns the *same* type as its `len` says" — projections are precisely the
   missing expressiveness, and the language's stated main goal
   (`docs/contributing/main-goal.md`: interfaces + reactivity + styles)
   explicitly wants interfaces.

---

## 8. Compatibility Fallout

- `MethodTable` becomes a *query result*, not an exclusive table. The change is
  contained in `crates/hir/src/context/types/methods.rs` and its consumers in
  the type checker; the fallback "inherent method of the concrete type" path is
  preserved so all existing program behavior is unchanged.
- Call-site *monomorphization* now must discharge bounds first. Every existing
  generic call site is unaffected (they have no bounds), but the monomorphizer's
  `in_progress` cycle detection needs to cover "same function, same args,
  different-bounds context."
- `assert_no_generic_non_functions` can be *reached with bounds*: nothing in
  this extension compels alias/stylesheet support, but the *tension* doc
  ([`07`](07-tensions.md) section 6) schedules alias support explicitly as the
  natural companion once bounds exist, because rejection reasons for bounds
  and for aliases share a root cause (nested generic contexts).
- Diagnostic expectations in `tests/type_checker.rs` may drift where an
  *inherent-method* error and a *no-such-method-on-concrete-type* error now
  route through "method lookup query returned no answers". The messages are
  preserved via the `QUERY-MISS` hook (overview section 5.2: "let an extension
  refine the answer").

---

## 9. Summary: What the extension buys, what the core pays

- **The extension buys**: bounded generics, interfaces, associated types,
  coherence, static and dynamic dispatch, supertraits, defaults.
- **The core pays**: one new predicate family (`satisfies`/`method`/
  `normalize`) hosted by rules, two new term families (`trait_ref`,
  `projection`, `bound_holder`), two pool slots, the projection rewrite rule in
  the normalizer, and the reification of impl-records/vtables at the driver.
- **The core refuses to pay**: any hardcoded meaning for `Display`, any vtable
  instruction, any orphan-rule arithmetic, any "trait" match arm anywhere in
  preexisting machinery.

Next: [`02-higher-kinded-types.md`](02-higher-kinded-types.md) — how kinds enter
the type representation without making the core know about `Functor`.