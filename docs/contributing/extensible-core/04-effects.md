# Effects as an Extension

> Series: *extensible-core*. Assumes [`00-overview.md`](00-overview.md).
> Complements [`01-traits-and-associated-types.md`](01-traits-and-associated-types.md)
> (evidence passing is dictionary passing with a different hat) and
> [`05-comptime.md`](05-comptime.md) (evidence is compile-time-specialized when
> statically known).

---

## 1. The Feature, Stated in the Language's Own Terms

Slynx has an **effect stub already in the tree.** The attribute pipeline
accepts and stores `@capabilities("fs", "io", "net")` — `HirAttributeKind::
Capabilities(Vec<SymbolPointer>)` in `crates/hir/src/model/declarations.rs`,
parsed in `crates/hir/src/builders/attributes/mod.rs`. Nothing consumes it.
This doc is the specification that stub was waiting for.

The feature has three demands the brief makes explicit:

1. **Effects must be *user-defined*:** the core ships no list of `fs`, `io`,
   `net`, `gpu` or any other capability. Language users (or the language's own
   `lib/` bindings, like `lib/std/color.slx`) define what an effect is.
2. **Effects can attach to *data*, not only to functions:** a struct `with gpu`
   can only be *created, read, and destroyed* inside a context that is
   `gpu`-enabled; the effect is in the *type of the value*, not (only) in the
   signature of the functions that use it.
3. **The semantics of "capable" contexts are a *rule in the engine*** — the
   core's *term + goal + driver-instrument* machinery, not a hardcoded list of
   strings.

The interesting software problem is neither "runtime continuations" nor
"algebraic effects" as a feature; it is **plumbing enforcement**: how a
compile-time *proof obligation* (value creation, value use, function call)
gets attached to code, and how the compiler *erases* it when the obligation is
statically discharged (the zero-cost case).

---

## 2. The Mechanism in Real Systems

### 2.1 Koka: the compile-time-first model (evidence passing)

Koka's function type carries an **effect row**: `foo : int -> <exn, fileio> int`
means "may raise, may touch files." Rows are *open* (`<exn | r>`) for
polymorphism over ambient effects, and handlers are declared with `with`
blocks. The mechanism that matters:

When a handler is **statically known** (the `with` block is lexically in scope
and the handler is a constant), Koka compiles the `perform`... `handle`
interaction into **direct calling** — the handler's method is called directly,
`with` desugars into closure capture, and the "effect invocation" is an
ordinary function call with **no allocation and no continuation capture**. This
is the zero-cost case, and it is the *same* mechanism as dictionary passing in
the traits doc (section 2.1): an effect method is a dictionary entry, and
statically-known-dictionary ⇒ direct call.

When the handler is **dynamically chosen**, Koka falls back to a runtime
evidence path (a "prompt"/continuation marker on a stack, plus capturing
continuations in heap-allocated frames). The operating cost of that path is
*real and opt-in*: it only occurs where the handler is genuinely computed at
runtime, never for statically-known cases. Koka's design goal was precisely to
make the common (static) case indistinguishable from no effects at all.

### 2.2 OCaml 5: the runtime model

OCaml 5 implements effects via **delimited continuations with segmented
stacks**. Mechanics: `perform` captures the continuation (a one-shot snapshot
of the execution stack, stored in a Fiber), the handler runs on a fresh
segment, and `continue` restores it. Every capture involves allocating a
continuation and switching stacks; the runtime's GC is modified to scan
suspended stack segments. This is a *powerful* and *general* runtime facility —
it handles deep, dynamically-chosen, resumable effects uniformly — but it is a
*feature with a runtime*: it cannot be erased when the handler is static, it
needs GC support, and it costs allocation per capture.

**The design decision for Slynx is visible already**: the *default* mode should
be Koka's compile-time-first evidence passing, because (a) the vast majority of
capability uses in a UI/IO language — "this function may call `fs`" — are
statically known, (b) it erases to nothing, and (c) it requires no runtime
features at all. The *escape hatch* (dynamic handler values) is a documented,
*optional* addition that lives outside this extension's compile-time model (it
needs a runtime representation; see section 7).

### 2.3 Effects attached to data: `ST`, Frank, Effekt

The tricky demand is #2 — "the effect lives in the *type of the value*." Three
real systems show the shape of the answer:

- **Haskell's `ST` monad.** `ST s a` carries a **region variable `s`**, and
  `runST :: (forall s. ST s a) -> a` *skolemizes* `s` so no `ST s` value can
  escape its thread. The mechanism is **a proof obligation in the type**: the
  type "holds" a phantom `s`; the type checker ensures the region's life
  cannot cross the boundary. At runtime `ST` is *erased* (an `ST s a` value is
  actually a `RealWorld -> (RealWorld, a)` function). The lesson: **a
  capability can be carried as a term inside a type, enforced by the type
  checker, and erased at runtime** — exactly what the core's term tree and the
  comptime-erasure story make available to Slynx.
- **Frank (Lindley et al.)**: **abilities** — a function explicitly declares the
  handlers it is parameterized on (`{FileIO, Stdout}`), and effect invocations
  (`!`) are resolved against the enclosing handler abstraction, statically.
  Frank restricts to **complete handlers** (no scoped/delimited control) to keep
  resolution decidable. This is the "effects as an implicit, statically-resolved
  argument" model — dictionary passing again.
- **Effekt**: **rigid capabilities** — handlers are *named values* passed
  explicitly (capability-passing style). The cost is threading capability
  parameters by hand; the payoff is that capabilities can be *scoped* and
  *swapable* naturally.

Slynx's synthesis, stated in one sentence: **an effect capability is a
*dictionary/evidence term* nested in a *type term* (data-attached) or *row
term* (function-scoped), and its enforcement rules are engine goals; the
compile-time-first lowering erases the evidence where statically known.**

---

## 3. What Slynx Already Has

- **The seed:** `@capabilities(...)` parsed → stored → unused
  (`crates/hir/src/builders/attributes/mod.rs`,
  `crates/hir/src/model/declarations.rs`). The attribute pipeline already does
  registration-from-source; the data just needs a consumer.
- **Lang items / builtins** (`crates/hir/src/context/lang_items.rs`, the
  `@builtin` mechanism): the compiler already resolves "compiler-known" names
  through registries — the pattern an effect's **standard capabilities** would
  use (a `cap fs` declaration in a stdlib lives as a lang-item-adjacent
  binding).
- **Function types carry nothing yet.** `HirType::Function`
  (`crates/hir/src/model/types.rs:390`) wraps a `FunctionType` with just
  `args` (a `SmallVec`) and `ret` — no effect row. This is the primary *shape*
  change in the extension (a row term slot), not a language semantic.
- **The IR flags side effects** — `impure_instructions` in
  `crates/ir/src/ir.rs` — which is the downstream anchor the effects extension
  hangs instrumentation on.

---

## 4. The Extension Design

```text
extension "effects":
  order: 60             # after terms, engine, driver; uses comptime's reify for evidence

  declaration_kinds:
    effect     ::= data(name, operations)      # an effect NAMES operations
    handler    ::= data(effect_ref, methods, scope_kind)  # "how to serve effect X here"

  type_terms:
    effect_row ::= arity: N    kind Type   # <fs, io> attached to a function type
    capability ::= arity: 1    kind Type   # "this VALUE's type carries effect-capability C"
    handler_ref::= arity: 2    kind Type   # a statically-resolved handler implementation

  predicates (goal form in the engine):
    ambient_capability(Cap, Row)          # proving "the ambient context has Cap"
    handled(Effect, Handler, Scope)       # is this effect served by a live handler?
    provably_static(Handler)              # can the driver erase evidence (zero-cost case)?

  driver hooks:
    ON FunctionCall, "effects":
        BEFORE-LOWER:  resolve which capabilities are out of ambient scope → error
        AFTER-LOWER:   if a capability is dynamic, keep a handler_ref argument;
                       create it via comptime reify (there is no "handler runtime
                       alloc" in the static path)
    ON "value creation", "value use":
        the extension "sees" every object construction / reference of a
        capability-typed value, and consults ambient_capability.
        ┌ Building a GpuBuffer in a non-gpu context: compile error (proof failed)
        └ the check is a *goal*, not a string list: user-defined effects behave
          exactly like fs/io/net.
    REIFY: turn a provably-static handler entry into a direct call,
           erasing the evidence argument. (Zero-cost case.)

  well_formedness:
    capability_declared(effect)           # "fs" must be DECLARED somewhere before use
    scoped_handle(handler, fn_range)      # handler lifetime is textual; no escape
```

### 4.1 The three enforcement points (this is the whole engineering)

Effects on data ("a `GpuBuffer` can only exist where `gpu` is available") are
not one rule — they are three, and the doc's `@capabilities` seed data
structure suggests the road:

| Point | Rule (goal) | Hook |
|-------|-------------|------|
| Value **creation** | `ambient_capability(gpu)` must hold where a `GpuBuffer` literal/constructor appears | `ON object-construction` |
| Value **use** | reading/destroying a capability-typed value requires the ambient capability | `ON user-of-captured-type` |
| Function **call** | the callee's `effect_row` must be *entailed* by the caller's ambient row (`<fs> ⊆ <fs, net>`) | `ON FunctionCall` |

Each of these is an **engine goal with a driver hook at a stable position** —
nothing about the *core* changes when the user invents `effect coin_flip`:
the rules are registered, the hooks are registered, the heap of "features" is
just *more rules + more terms*.

### 4.2 The zero-cost erasure (evidence passing, compile-time-first)

Following Koka: when `provably_static(handler)` holds, the handler is *not*
passed at runtime — the effect operation call is rewritten (at `REIFY`) into a
direct call to the handler's method, exactly as a monomorphized dictionary
disappears in the traits doc. When the handler is genuinely dynamic, the
extension *keeps* a `handler_ref` argument through lowering (the IR already
represents function references and opaque data — `crates/ir/src/types/
irtype.rs` has `Function` types and pointers); the cost is then opt-in, and the
*dynamic* case is served by an ordinary function-pointer call, which the
backend already handles. **No new IR instruction for effects.** No `impure` flag
new. The IR stays stupid; the extension is the intelligence.

### 4.3 Why comptime-adjacent staging is the right tool

The overview (section 3.2) gives terms a *stage* flag. Effects lean on it: the
evidence ("which handler am I inside?") is *compile-time-known* in the common
case, and the extension asks the driver to *reify* that known value into the 
lowered call. The "is this static?" question is the comptime flavor of *"can
the compiler settle this now?"* — which is the same question comptime
([`05`](05-comptime.md) section 3) asks, served by the same worklist driver.

---

## 5. The Concrete Example (`gpu` — user-defined, data-attached)

```slynx
/*
   A user defines an effect. No compiler change anywhere.
   This is the entire claim of the extension, in miniature.
*/
effect gpu {
    operation sync();
}

/* stage A: data carries the capability */
object GpuBuffer {
    let bytes: [u8];
}
with gpu on GpuBuffer;          // creates: GpuBuffer is a `capability(gpu)`-typed value

/* stage B: a function whose row includes gpu, so it may create/use one */
func render(buf: GpuBuffer) : gpu void {
    sync();                     // ok: gpu ∈ <gpu>
    buf.bytes.push(...);        // ok: creating/using a gpu-captured value
}

/* stage C: a non-gpu function is *rejected* for touching the value */
func pure_log(buf: GpuBuffer) : void {
    sync();                     // ERROR: goal ambient_capability(gpu) fails
}
```

Mechanically: `render`'s function type gains `effect_row: <gpu>`; calling
`render` from a `void`-row context without surrounding `with gpu` fails the
goal `ambient_capability(gpu, <>)`; a `with gpu { ... }` block installs a
handler whose record reifies into `handler_ref` and lets `sync()` lower to a
direct call to the static handler. `lib/` itself defines `fs`, `io`, `net` the
same way (the `@capabilities("fs",...)` attributes in existing examples just
start naming them). The *builtin* status of fs/io/net is a *convention of the
builtins extension* (`std`), never a core truth — the overview's section 4
exception, honored here verbatim.

---

## 6. Alternatives Considered and Refused

1. **Runtime effect handlers (OCaml-5-style) as the primary mechanism.**
   Refused: Slynx has no GC-tagged segmented stacks, no delimited
   continuation runtime, and its codegen story is small/portable. Costing every
   effect with a continuation capture when statically-known cases could be
   free is the reverse of the compile-time-first order. *Documented as the
   optional dynamic-handler escape hatch* (section 7) for when the language
   really needs resumable/dynamic effects.
2. **A fixed, built-in capability set (`fs`, `io`, `net`, `gpu`) in the core.**
   Refused — this is the *original sin* this extension exists to prevent. The
   stub already looks tempting because `Capabilities(Vec<SymbolPointer>)`
   parses strings; the erasure path confirms why it must stay a rule.
3. **Effekt-style rigid capabilities as the *only* model.** Refused as primary:
   manual capability threading is the kind of ceremonial tax Slynx rejects in
   the typestate doc's refusal of PhantomData. It remains *available* (the
   handler-scope rules are permissive enough that a user can be as explicit as
   Effekt); it is simply not the default advice.
4. **Frank-style **complete-handlers-only**.** Refused as the *floor*: the
   "scoped use and release" (`handler { ... }` scope) pattern that Frank bans
   for decidability is exactly what UI code wants (bind an effect for a widget
   subtree); we keep scoped handlers (Mungo/Plaid-sibling scoping rules), and
   accept the associated *static-resolution-only* constraint, which is what
   compile-time-first means anyway.

---

## 7. The Dynamic Escape Hatch (documented, not implemented)

A genuinely dynamic handler ("choose handler at runtime based on a flag")
cannot be erased. The compile-time-first design *permits* it as a consciously
wider step: the handler implementation becomes a value of a `function`-typed
field, a `handler_ref` parameter is threaded at lowering (evidenced by the
existing pointer/`Function` IR types), and the call *is* the function-pointer
call the backend already emits. This trades zero-cost for an extra indirection
per operation, *only where chosen*. The doc's position: ship static-first; the
dynamic path is an IR-free, runtime-less *extension of the extension* enabled
by the same rules — the one missing piece (a "suspended continuation") is a
runtime feature and thus out of the core's remit.

---

## 8. Compatibility Fallout

- **`HirAttributeKind::Capabilities` finally has meaning.** The attribute now
  translates into a `capability` term + `ambient_capability` goals; previously
  ignored `@capabilities` annotations on existing programs will *begin to
  type-check* (they declare what the program claims). Programs without the
  attribute are unaffected.
- **Function types grow an effect row slot.** Every existing `HirFunctionType`
  is a row `< >`; the change is additive and invisible (row-entailment of an
  empty row always holds).
- **The driver's `ON FunctionCall` and `ON object-construction` hooks are
  new positions** — the only place where the *core* grows (hooks are the core's
  syscalls, per overview 5.2). They are additive, rarely-added, and versioned.
- **`impure_instructions`** (`crates/ir/src/ir.rs`) remains the IR's own
  side-effect marker and is *not replaced* — an effect call is simply an
  impure instruction, unchanged machinery.

---

## 9. Cross-References

- Evidence passing ≈ dictionary passing (traits doc section 2.1); provable
  static erasure is the same `REIFY` operation (overview 3.5, comptime
  doc section 3).
- The `ST`/region lesson (capability-in-type, erased at runtime) is the HKT
  doc's staged-kind story applied to values.
- The `07-tensions` doc uses effects as the *prime exhibit* that the core must
  not choose between static and dynamic dispatch: it is a per-call-site policy
  of the extension, not a core decision.

---

Next: [`05-comptime.md`](05-comptime.md) — the interpreter and reification that
turn "types as values" from a dream into a scheduled driver loop.