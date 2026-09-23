# Higher-Kinded Types as an Extension

> Series: *extensible-core*. Assumes [`00-overview.md`](00-overview.md).
> Companion to [`01-traits-and-associated-types.md`](01-traits-and-associated-types.md);
> the two meet where trait bounds range over type *constructors* (`Functor f =>`),
> so this doc is required reading before `07-tensions.md` section 5.

---

## 1. What Higher-Kinded Types Are, and Why Slynx Wants Them

Ordinary types classify *values*: `int` classifies `5`, `Vec<int>` classifies
`[1, 2]`. A constructor classifies *types*: `Vec` is "given any type `T`, the
type `Vec<T>`." A **higher-kinded** variable is one that stands for *a whole
constructor*: in `func map<f<_>, a, b>(app: f<a>, fn_(x: a) -> b): f<b>`, the
`f` is not a type — it is a type *function* of one argument.

The languages that ship this (Haskell, Scala 3, Reason/OCaml via modules)
use it to abstract over "structures that hold things": `Functor` (mapping over
the contents), `Foldable`, `Traversable`, and — closer to Slynx's goals —
abstract interfaces for *state machines* and *reactive sources*, which are
exactly what a UI/typestate language wants to share between `Option<T>`,
`Vec<T>`, a `Stream<T>`, and a `Socket<Variant<T>>`.

But there is a *second*, quieter reason Slynx in particular cares, and it is
the reason the typestate doc ([`03`](03-typestate.md)) and this doc are
siblings: **the classic "typestate via `PhantomData<State>`" trick requires a
conceptually higher-kinded thing** — `State` is a token whose only job is to be
a type parameter of kind `*`, and *storing it in a wrapper* (`Foo<State>`) is
the kernel of a higher-kinded application. The brief explicitly refuses the
PhantomData approach, but the *ability to say "type application" cleanly* still
underpins it. HKT and typestate both live and die on whether type *expressions*
are first-class trees with arities.

---

## 2. The Mechanism in Reference Systems

### 2.1 GHC/Haskell: a second level of typing

GHC's internal types carry **kinds**, and kinds are themselves a well-typed
language:

```
*            : the kind of ordinary types            (int   :: *)
* -> *       : the kind of one-arg constructors      (Maybe :: * -> *)
(* -> *) -> * : constructors over constructors        (a "Functor-ish" shape)
```

The mechanism that matters is **kind inference**: when the compiler sees
`f a`, it must check that `f` accepts one argument, i.e. `kind(f) = * -> *`. It
runs exactly the same unification algorithm it runs for types, but one level
up — kind variables are just type variables inhabiting the kind level. GHC even
generalizes this into *levity polymorphism* and *kind polymorphism*; the lesson
is not "GHC is complicated" but **"the algorithm is reusable; the plumbing is
just a second, smaller type system."**

**Dictionary passing** is the runtime story and it is worth stating precisely:
a class constraint `Functor f =>` compiles to a hidden parameter carrying the
class's *dictionary* for the concrete constructor `f`, exactly like the
generics/trait dictionary of [`01`](01-traits-and-associated-types.md) section
2.1. Nothing about higher-kinded *types* requires new runtime machinery —
dictionaries still hold function pointers; the only difference is that a
dictionary is now keyed by a *constructor* (`Maybe`) rather than a *type*.
Because Slynx monomorphizes anyway, the HKT extension inherits the static
dispatch story for free: `map` specialized at `f := Vec` knows `Vec::map`'s
address statically.

### 2.2 Rust: the compromise (GATs)

Rust has no higher-kinded types. It ships **GATs — generic associated types**:
`type Item<'a>` inside a trait, i.e. a *family* of associated types indexed by
a lifetime (or, in general, by type parameters). The canonical case is
`trait Iterable { type Life<'a>; }` so that borrowing `&'a Self` yields a
borrow-lifetime-matched iterator type. GATs give Rust 80% of the "data
structures that are families over parameters" use case without two-level type
inference. The *internal* cost is real: GATs are the single most complex corner
of Rust's trait solver (they interact with the projection normalization of
section 2.3 in the traits doc, and with region inference), because a projection
`<T as Iterable>::Life<'x>` must be normalized *for each instantiation of the
indexing parameter*.

### 2.3 Scala 3 / Dotty: type lambdas as the middle ground

Scala 3 synthesizes missing type-constructor applications with **type lambdas**,
written `[X] =>> F[X]`. The compiler internally normalizes a "partially applied"
type application into a lambda form. This is the pragmatic engineering answer:
*the user writes real higher-kinded syntax; the compiler lowers it to a small,
boring theory of lambdas + application.* Dotty's internal type system is, at
bottom, a lambda calculus of types, which is exactly the moral of section 3.

---

## 3. What the Core Provides: Kinds as Structural Plumbing

The overview (section 3.2) staked the position: the core *measures arity*, it
does not *semanticize* constructors. Concretely:

1. **Kind vocabulary is closed and tiny** — `Type` (`*`), `Fn(k1, k2, ...)`
   (constructor kinds), `Universe` (the kind of kinds), plus a `Var` kind for
   kind-inference holes. The core can **infer** and **unify** kinds because
   these are the same operations it already does on terms. It is, in GHC's
   terms, "the second level," always available, always fast.

2. **Every term node exposes its kind.** A `data` node has the kind derived
   from its arity in the descriptor pool; an `apply` node's kind is computed
   from its head's kind minus one argument; extension nodes report kinds
   (children + kind is the whole contract, per overview section 5.3).

3. **The closed `[DedupPoolId<HirType>; 8]` generic array is *the* thing that
   dies.** The 8-element fixed array in `HirType::Reference`
   (`crates/hir/src/model/types.rs`) is an artifact of "generics = a bounded
   tuple of types", not of any design; in the term tree an application is a
   *node* with an arbitrary child list. The kind of an application is
   *computed*, so `f<a>`, `f<a, b>`, and a completely unsatured `f` are just
   three different (term, kind) pairs, not three enum variants.

4. **Staging (overview 3.2) is kind-compatible.** A comptime-known type is a
   term with stage `CompileTime`; nothing about kinds forbids a constructor
   whose *result* is compile-time. (Comptime doc [`05`](05-comptime.md) uses
   this: `comptime T: type` is a *typed* binding whose type is a kind.)

When the user writes **no** higher-kinded syntax, nothing changes: ordinary
types are all kind `*`, kind inference is trivial, and the cost is one field
per term.

---

## 4. The Extension: `f<_>`, Type Lambdas, and Constructor Bounds

```text
extension "hkts":
  order: 40            # after terms (arities exist), before traits uses constructor bounds

  syntax:
    kind_spec   ::= f<_>                      # "one-arg constructor" annotation
    type_lambda ::= [X] =>> Apply(head, X)    # Dotty-style: partially applied constructor
    forall_cstr ::= forall f in Fn($, *)  :   # quantifier over constructors, only in bounds

  type_terms:
    type_lambda  ::= arity: 2   kind Fn(k1, k2)   # a lambda node: binder + body
    apply        ::= arity: N   kind computed     # already core; now kind-aware
    kind_asis    ::= arity: 1   kind Universe     # "this constructor has kind X" (a proof fact)

  predicates (registered in the engine, clause-free: proven by kind computation):
    kind_of(Term) -> Kind                 # what the term levels to (the "second type system")

  laws (registered into the normalizer):
    beta: apply(type_lambda(X, body), arg) ~> body[X := arg]   # Dotty's normalization step

  rules into the traits extension (by manifest order > traits.core):
    satisfies(f, Functor)  :-  kind_of(f) = Fn($, *),  <user impl of Functor for f>,
                              and the trait's methods type-check at kind level.
```

The three real deliverables of the extension:

1. **User-facing kind annotations** (the `f<_>` syntax), which feed the
   `kind_asis` terms and produce diagnostics.
2. **Type lambdas with beta-reduction** registered as a normalizer rule. This
   is Dotty's algorithm lifted almost verbatim: never leave a *partial*
   application in a term — write it as a lambda, beta-reduce lazily during
   normalization. Because the core normalizer *already* applies registered
   rewrite rules (overview 3.2), the HKT extension adds one rule
   (`beta`), not a new solver.
3. **Constructor bounds,** which plug into `satisfies(f, Functor)` — a goal
   whose first argument is now of kind `* -> *`. The engine does not care: 
   `satisfies` is *already* a rule over arbitrary terms, and the traits
   extension already provided dictionary-style answers. The only new thing is
   that one level of the unification happening *inside* the goal is now kind
   unification.

**The GAT problem does not disappear; it becomes an engineering choice.** The
projection machinery of the traits doc *can* implement GATs (it is, after all,
"indexed projection"), but the doc's recommendation is to *not* ship GATs
first: the lambda-normalization path covers the same expressiveness for Slynx's
short-term needs (typestate, streams, collections) with far less solver
complexity, and GATS can be layered later as "projections whose normalization
depends on index instantiation" — a *variation of the projection rewrite rule*,
never a core change.

---

## 5. A Note For the Skeptic: Isn't This Just Overkill?

Fair question, and the answer is the two concrete users:

1. **Typestate is the lock-in.** [`03`](03-typestate.md) section 4 will use an
   automaton whose *states* are type terms in a state-token constructor. The
   whole "in-place transitions" story is a story about *type-level value*.
2. **`comptime` type-level programming interacts with kinds.** Once types can
   be *computed*, "is this type expression well-formed" = "does it kind-check",
   which is the same query (`kind_of`) — the comptime extension reuses it
   (`05` section 3).

If neither were compelling, the design could stop at *kinds as structure
measurement only* (core terms) and never ship the `f<_>` syntax. The doc
deliberately makes that a *configuration point*, not a fork: the cost of the
syntax is bounded by the extension, and the core gain persists either way.

---

## 6. Compatibility Fallout

- **`crates/hir/src/model/types.rs`** `HirType::Reference { generics: [..; 8] }`
  collapses into a generic-application term with a real list. All consumers
  (`substitute_types` in `crates/hir/src/generics.rs`, the monomorphizer's
  `MonomorphizationKey`, `crates/hir/src/helpers/views/types.rs` rendering,
  `DedupPoolId<HirType>` equality) must move from "compare 8-sized arrays" to
  "compare term-list ids." This is the highest-touch, lowest-risk change in the
  whole series: it is arithmetic, not semantics.
- **The monomorphizer's specialization tables** are keyed on *concrete arguments*.
  Higher-kinded *arguments* can now be a lambda, so the key becomes
  *term-tree-hashable* rather than *"all leaves are concrete types"*. The cache
  in `crates/monomorphizer/src/lib.rs` already hashes keys; the key type grows.
- **Type names in errors:** `f`'s name rendering for a constructor term — the
  `name()` fn in `types.rs` — gains a case for lambda/application nodes, but via
  the term protocol (children + kind), not a match on a variant.
- **No runtime change whatsoever.** The IR `IRType` stays fully concrete
  (`crates/ir/src/types/irtype.rs`); the HKT ext is erased by the monomorphizer
  like every other type-level construct. The IR never sees a lambda.

---

## 7. Alternatives Considered and Refused

1. **GATs first (Rust's compromise).** Refused as the *first* move: GAT
   normalization couples projection solving with index-region inference, which
   is the hardest known corner of Rust's solver. For a UI-first language the
   use case ("borrow-indexed iterator types") is real but orthogonal; typestate
   and collections are served by plain higher-kindedness. Layered-in-later
   remains open (it is just projection-rule variation).
2. **No higher-kinded types at all (pre-inference-economy lure).** Refused:
   typestate (doc 03) and reactive/composable UI types need to abstract over
   "the thing that holds T." The freedom to say `map<f<_>, ...>` is cheaper
   than every workaround the language would otherwise force (codegen a separate
   interface per concrete constructor — a combinatorial explosion).
3. **Full GHC-style *kind polymorphism* from day one.** Refused as scope creep:
   kinds-in-the-core are closed (overview 3.2) precisely to keep `kind_of`
   tractable; *polymorphic* kinds would make the kind level itself higher-kinded
   and re-open the recursion. The core document is explicit that the kind
   vocabulary is closed and this doc does not reopen it.
4. **Structural typing over constructors** (Go-style "any container with a
   `map`"). Refused for the same reason the traits doc refused structural
   interface: no coherence, weaker specialization, and its real users (typestate
   transitions) need *nominal* identity anyway.

---

## 8. Cross-References

- The **typestate** doc ([`03`](03-typestate.md)) section 4 uses constructor
  terms; section 2 of this doc explains why the PhantomData style secretly
  needed a constructor.
- The **comptime** doc ([`05`](05-comptime.md)) section 3 reuses `kind_of`
  with staged terms.
- The **tensions** doc ([`07`](07-tensions.md)) section 5 answers the headline
  tension this used to cause — "single-kind checker vs. two-level types" —
  by showing that kinds-in-terms makes both the closed `*`-only language and the
  higher-kinded language run on one representation.

---

Next: [`03-typestate.md`](03-typestate.md) — the dataflow extension that turns
the ownership analysis's small lattice into a user-configurable automaton.