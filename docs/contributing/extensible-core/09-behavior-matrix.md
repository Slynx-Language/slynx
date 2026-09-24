# Phase 0 — The Behavior Matrix

> Series: *extensible-core*. This document is the deliverable of
> [Phase 0 of the implementation order](08-implementation-order.md#2-phase-0--trust-the-harness):
> the authoritative table of "program → expected outcome" that every later phase
> is judged against mechanically. It records the compiler's **current, verified
> behavior** (HEAD + working tree, `2026-09-23`). Nothing below is a wishlist:
> every row reflects an existing test gate that is green today.

---

## 0. The Contract (the "hook")

Every phase in [`08`](08-implementation-order.md) must finish with **this
matrix green**, plus whatever *new* entries the phase itself adds. Concretely:

1. `STD_PATH=./lib/std cargo test` passes for every test binary listed in
   §2 — that is the operational definition of "matrix green".
2. A phase may **only** flip a row from `REJECT` → `ACCEPT` (a relaxation) or
   add rows; a flip from `ACCEPT` → `REJECT` (a tightening) is permitted only
   when the row is captured by a `// xpass:` marker and is announced (a
   *deliberate* validation landing, not an accident).
3. When a phase changes an outcome, it must update **both** this doc *and* the
   machine-readable markers (`// xfail:` / `// xpass:` in `examples/generics/`
   and `examples/enums/`), because the harness encodes this matrix as comments.
4. Marked example files are the machine-readable encoding of the matrix for the
   example directories; this document is the human-readable index plus the
   diagnostic/IR acceptance criteria the tests assert.

---

## 1. How to Run the Gates

```bash
# Full behavioral matrix (all test binaries):
STD_PATH=./lib/std cargo test

# Selected crates' unit suites (term/type invariants):
cargo test -p slynx-hir
cargo test -p slynx-monomorphizer
```

Environment: `STD_PATH=./lib/std` points the compiler at the prelude
(`lib/std/color.slx` is the entire prelude surface today). The harness helpers
live in `tests/common/mod.rs` (`compile_source`, `compile_ok`, `load_source`,
`load_context`, `STD_PATH`).

---

## 2. Gate Inventory

| Test binary | Role in the matrix | Primary phase it anchors |
|---|---|---|
| [`tests/generics.rs`](../../../tests/generics.rs) | Compiles every `examples/generics/*.slx`; outcome driven by `// xfail:`/`// xpass:` markers. Encodes **specialization**. | 1 (terms), 4 (traits), 8 (comptime) |
| [`tests/enums.rs`](../../../tests/enums.rs) | Same marker harness over `examples/enums/*.slx`. Encodes enum lowering. | 1, 8, 4 |
| [`tests/monomorphizer.rs`](../../../tests/monomorphizer.rs) | Specialization counts, mangling shape, dedup, dead-code, arity/cycle behavior on inline sources. | 2 (engine `specialize`), 7 (driver) |
| [`tests/ownership.rs`](../../../tests/ownership.rs) | Move/borrow acceptance on inline sources. Encodes **borrow semantics** today. | 3 (flow/ownership port) |
| [`tests/move_semantics.rs`](../../../tests/move_semantics.rs) | Compiles `examples/move_semantics/*.syx`; the four-folder corpus of writes/borrows/moves. | 3 |
| [`tests/type_checker.rs`](../../../tests/type_checker.rs) | Type checking accept/reject + exact diagnostic strings. | 1 (unifier), 4 |
| [`tests/functioncall_invalid_args.rs`](../../../tests/functioncall_invalid_args.rs) | Call-arity rejection with structured error payloads. | 4 |
| [`tests/imports.rs`](../../../tests/imports.rs) | File/selective/alias imports + wrong-module rejection. | (arena/module loading) |
| [`tests/externs.rs`](../../../tests/externs.rs) | Compiles every `examples/externs/*.slx` (extern objects/functions/statics). | — |
| [`tests/stylesheet_uses.rs`](../../../tests/stylesheet_uses.rs) | Compiles every `examples/styles/*.slx` (stylesheet `uses` inheritance). | 4 (generic aliases/styles) |
| [`tests/objects.rs`](../../../tests/objects.rs), [`tests/obj_methods.rs`](../../../tests/obj_methods.rs) | Object/method sources load through module+HIR only (STAND-IN — codegen not yet hoisting objects). | 3, 10 (see §4) |
| [`tests/arrays.rs`](../../../tests/arrays.rs), [`tests/tuples.rs`](../../../tests/tuples.rs), [`tests/booleans.rs`](../../../tests/booleans.rs), [`tests/while.rs`](../../../tests/while.rs), [`tests/if_expression.rs`](../../../tests/if_expression.rs), [`tests/number_systems.rs`](../../../tests/number_systems.rs), [`tests/variable.rs`](../../../tests/variable.rs), [`tests/common_comments.rs`](../../../tests/common_comments.rs) | Single golden examples compile to IR; `while`/`if_expression` additionally assert IR shape. | 1, 7 (IR production) |
| [`tests/component_type_mismatch.rs`](../../../tests/component_type_mismatch.rs) | Component prop assignment must be type-rejected. | 1 (unifier), 4 |
| [`tests/compilation_output.rs`](../../../tests/compilation_output.rs) | `compile()`/`build_stages()`/`compile_code()` **write-behavior** (no file written until `write()`; `sir`/`hir`/`ir` dumps). | 7 (driver) |
| [`tests/parser.rs`](../../../tests/parser.rs) | Empty placeholder (parser is exercised transitively). | — |

---

## 3. The Matrix

### 3.1 Pipeline & IR production (golden examples)

`ACCEPT` rows produce IR and, where the test asserts it, a `.sir` sidecar only
after `write()`. Diagnostics/IR-shape acceptances are listed per row.

| ID | Program | Expected today | Notes |
|---|---|---|---|
| BM-101 | `examples/arrays.syx` | ACCEPT → IR | array/slice lowering |
| BM-102 | `examples/booleans.syx` | ACCEPT → `.sir` | bool primitives |
| BM-103 | `examples/commonComments.syx` | ACCEPT → `.sir` | lexer comment handling |
| BM-104 | `examples/ifExpression.syx` | ACCEPT → IR containing `main` and a `Cbr` | if/else as expression lowered to conditional branch |
| BM-105 | `examples/numberSystems.syx` | ACCEPT → `.sir` | int literal bases (`0x`, `0b`, `0o`) |
| BM-106 | `examples/variables.syx` | ACCEPT → `.sir` | variable declarations/bindings |
| BM-107 | `examples/while.syx` | ACCEPT → `.sir` | while loops |
| BM-108 | `examples/tupleAccess.syx` | ACCEPT → `.sir` | tuple access; regression: concrete-struct-in-tuple must not panic `IRTypeNotRecognized` |
| BM-109 | `examples/tupleTwoObjects.syx` | ACCEPT → `.sir` | two same-typed objects in one tuple |
| BM-110 | `examples/tupleNestedObject.syx` | ACCEPT → `.sir` | nested `((Person, str), int)` |
| BM-111 | `examples/componentTypeMismatch.syx` | **REJECT** | assigning object `B` into a property typed as `A` fails type checking (regression test) |
| BM-112 | `examples/objects.syx` | ACCEPT-ADDITIVE (load + HIR only) | object types not yet recognized by codegen (STAND-IN) |
| BM-113 | `examples/objMethod.syx`, `objMethods.syx`, `objMethodStatic.syx` | ACCEPT-ADDITIVE (load + HIR only) | method resolution not yet implemented (STAND-IN) |
| BM-114 | `SlynxContext::compile()` (any source) | ACCEPT; **no** `.sir` file exists until `output.write()`; `output_path` is `source.with_extension("sir")` | compilation_output contract |
| BM-115 | `compile_code(path)` | ACCEPT; writes non-empty `.sir` immediately | write-behavior differs from `compile()` |
| BM-116 | `build_stages()` | ACCEPT; exposes `ir_text()` non-empty; `dump_path("hir"/"ir")` exist but files do **not** materialize until `write_ir()`/`into_output().write()` | driver-boundary contract (07) |

### 3.2 Ownership & move semantics (borrow model today = borrow-to-function-end)

`REJECT` rows carry the string `"moved"` in the error text unless stated.

| ID | Program | Expected today | Notes |
|---|---|---|---|
| BM-201 | `object Box{…}` ; `let a=Box(1); let b=a; let c=a;` | **REJECT** ("moved") | use-after-move through assignment |
| BM-202 | same as BM-201 | **REJECT** ("moved") | move-through-assignment is detected |
| BM-203 | `let a=Box(1); { let b=a; } let c=a;` | **REJECT** ("moved") | move in inner scope still invalidates outer use — borrow lives to function end; **will relax under Phase 3.2 (NLL)** |
| BM-204 | `let a=Box(1); let b=a; let r=&a;` | **REJECT** ("moved") | borrow after move detected |
| BM-205 | `let a=Box(1); let b=a;` (no further use) | ACCEPT | single move, end of use |
| BM-206 | `take(a)` then `let b=a;` | **REJECT** ("moved") | passing a value to a function is a move |
| BM-207 | `examples/move_semantics/references.syx` | ACCEPT | returning `&` to locals/fields; nested struct reference chain |
| BM-208 | `examples/move_semantics/valid_writes.syx` | ACCEPT | `&mut` + write through it (`*aref =` / `pref.name =`) |
| BM-209 | `examples/move_semantics/invalid_writes.syx` | **REJECT** | write through immutable `&` (`pref.name = ""`) is invalid |
| BM-210 | `examples/move_semantics/invalid_single_writer.syx` | ACCEPT (GAP) | `let mut p2 = p;` then `&mut p2` after a move-like binding **compiles** today; the file's own comment expects an error. Single-writer/move model not enforced. **Anchor for Phase 3.** |

### 3.3 Generics & monomorphization (`examples/generics/*`, inline sources)

All 30 example files **ACCEPT** today. Seven carry `// xpass:` markers
(documenting validations that should eventually reject). `REJECT` rows in the
inline gates are structured errors.

| ID | Program | Expected today | Notes |
|---|---|---|---|
| BM-301 | `generic_func.slx`, `generic_func_bool.slx`, `generic_func_float.slx`, `generic_func_str.slx` | ACCEPT | `identity<T>` over `int/bool/f64/str`; specialization per arg type |
| BM-302 | `generic_struct.slx`, `generic_struct_two_params.slx`, `struct_in_struct.slx` | ACCEPT | generic `object`, two params, nested generic struct |
| BM-303 | `generic_component.slx`, `generic_component_two_params.slx` | ACCEPT | generic `component` + prop specialization |
| BM-304 | `component_with_struct.slx`, `component_nullable.slx`, `component_collections.slx` | ACCEPT | component prop = generic struct / `Option<int>` / `[]int` & `[4]int` |
| BM-305 | `generic_style.slx` | ACCEPT | generic `stylesheet A<T>(x:T)` lowers with a generic object/component value — **Phase 4 lifts the alias/style restriction** |
| BM-306 | `dedup.slx` | ACCEPT; exactly **1** dead template + **1** `identity<int>` specialization | BM-mono: dedup of identical instantiations |
| BM-307 | `multi_param.slx` | ACCEPT; exactly **1** `second<bool,int>` specialization, mangled name has **4** `_` segments (`_<name>_<hash>` per generic) | BM-mono: mangling shape |
| BM-308 | `nested.slx` | ACCEPT; **2** dead templates; **1** `wrap<int>` + **1** `identity<int>` | BM-mono: nested calls instantiate both generics |
| BM-309 | `generic_over_array.slx`, `generic_over_vector.slx` | ACCEPT | `len<T>` over `[4]T` / `[]T`; literal unifies with param |
| BM-310 | `index_generic_value.slx`, `index_generic_struct_field.slx` | ACCEPT | index on generic value/field retyped after substitution |
| BM-311 | `nullable_return.slx`, `nullable_field.slx`, `option.slx` | ACCEPT | `Option<T>`-typed returns/fields; `option.slx` is the library module |
| BM-312 | `nullable_collection_type_arg.slx` | ACCEPT (**`xpass:`**) | `([]int)?` as a type argument compiles today; parse of the type arg was historically broken |
| BM-313 | `inferences.slx` | ACCEPT (**`xpass:`**) | `identity(42)` (no explicit type args) compiles today; historically panicked in codegen — generic inference must eventually work, not be guessed |
| BM-314 | `wrong_arity.slx` | ACCEPT (**`xpass:`**) | `second<int>(1,2)` compiles today; **arity validation is currently relaxed** — should eventually REJECT |
| BM-315 | `xpass_type_mismatch.slx` | ACCEPT (**`xpass:`**) | `identity<int>(true)` compiles; call-site type validation missing — **Phase 4 anchor** |
| BM-316 | `xpass_call_type_mismatch.slx` | ACCEPT (**`xpass:`**) | mismatched call args (`identity<int>(30.0)` etc.) compile and are silently reinterpreted — **Phase 4 anchor** |
| BM-317 | `xpass_component_prop_mismatch.slx` | ACCEPT (**`xpass:`**) | component prop values never checked against prop types — **Phase 4 anchor** |
| BM-318 | `xpass_struct_field_mismatch.slx` | ACCEPT (**`xpass:`**) | struct field values never checked against field types — **Phase 4 anchor** |
| BM-mono-1 | `alias A = B; alias B = A;` | ACCEPT-ADDITIVE (HIR builds) | **cyclic-alias detection missing**; should eventually REJECT (test marks the expectation) |
| BM-mono-2 | `second<int>(1,2)` inline | ACCEPT | generic arity tolerated (see BM-314) |
| BM-mono-3 | every specialization gate | ACCEPT | specialization counts, `_name_{:04x}` mangling, dead-code set, cycle detection — exact invariants of [`tests/monomorphizer.rs`](../../../tests/monomorphizer.rs) |

### 3.4 Enums (`examples/enums/*`)

20 of 25 files **ACCEPT**; 5 `// xfail:` files **REJECT**; 1 `// xpass:` file
accepts-with-gap.

| ID | Program | Expected today | Notes |
|---|---|---|---|
| BM-401 | `associated_single/multiple/mixed.slx` | ACCEPT | payload-bearing variants + `matches` |
| BM-402 | `raw_basic/raw_multiple/raw_and_valued/raw_valued/raw_valued_multiple.slx` | ACCEPT | raw variants, auto and explicit discriminants |
| BM-403 | `repr_int.slx` | ACCEPT | `enum …: int` repr |
| BM-404 | `struct_basic.slx`, `struct_multiple_fields.slx` | ACCEPT | struct-style payload variants |
| BM-405 | `generic_basic.slx`, `generic_multiple_params.slx`, `generic_enum_two_params.slx`, `generic_two_specializations.slx`, `generic_enum_function_signature.slx` | ACCEPT | generic enums, multi-param, two specializations at once, enum in function signature |
| BM-406 | `nested_enum_payload.slx` | ACCEPT | enum payload referencing another enum (on-demand flat layout) |
| BM-407 | `empty_enum.slx` | ACCEPT | dead empty enum must be skipped by hoisting (no crash) |
| BM-408 | `return_enum_payload.slx` | ACCEPT | returning payload variant (tag + payload slice) |
| BM-409 | `invalid_pattern.slx` | **REJECT** (`// xfail:`) | `matches` on a non-pattern RHS |
| BM-410 | `matches_on_non_enum.slx` | **REJECT** (`// xfail:`) | `matches` on a non-enum LHS |
| BM-411 | `payload_arity_mismatch.slx` | **REJECT** (`// xfail:`) | wrong payload argument count |
| BM-412 | `repr_str.slx` | **REJECT** (`// xfail:`) | only `repr: int` supported |
| BM-413 | `variant_unrecognized.slx` | **REJECT** (`// xfail:`) | referencing an undeclared variant |
| BM-414 | `payload_type_mismatch.slx` | ACCEPT (**`xpass:`**) | wrong-typed payload arg compiles today; literal builders ignore expected payload type — **Phase 4 anchor** |

### 3.5 Externs, imports, styles

| ID | Program | Expected today | Notes |
|---|---|---|---|
| BM-501 | all 18 `examples/externs/*.slx` | ACCEPT | extern objects/functions/statics; self-referencing and chained extern objects |
| BM-502 | `examples/imports/main.slx` | ACCEPT | file imports + `using … as …` alias (`styles.slx`, `another.slx` are import targets, not standalone gates) |
| BM-503 | `examples/imports/brace_import.slx`, `brace_alias_import.slx` | ACCEPT | selective brace imports with aliases |
| BM-504 | `examples/imports/test_wrong_module_import.slx` | **REJECT** | `BgGreen` not exported by `styles.slx`; selective import must not resolve across modules |
| BM-505 | all 11 `examples/styles/*.slx` | ACCEPT | stylesheet `uses` inheritance: direct/inherited, multi-parent, override, arg expressions/literals, order-independent declarations |
| BM-506 | `stylesheet_test.slx`, `component.syx`, `externs.syx`, `functionCall.syx`, `tuplas.syx` | ACCEPT | additional illustrative golden examples exercised transitively |

### 3.6 Type checker & expression diagnostics

| ID | Program | Expected today | Notes |
|---|---|---|---|
| BM-601 | `func bar():void {} func main():void { bar() }` | ACCEPT | call resolution across declaration order |
| BM-602 | `takes_int(true)` | ACCEPT (GAP) | argument-type validation not yet in builder — should eventually REJECT (**Phase 4**) |
| BM-603 | `func main():int { let x = 12; }` | **REJECT** | non-void function without a return value |
| BM-604 | `func main():void { let x = 12; }` | ACCEPT | non-expression tail statement allowed in void function |
| BM-605 | `while 10 { 0; }` | ACCEPT (GAP) | while-condition types not validated in builder — should eventually REJECT (**Phase 4**) |
| BM-606 | `while true { takes_int(false); }` | ACCEPT (GAP) | call-site arg validation missing (see BM-602) |
| BM-607 | `let pair = (10,20); pair.2` | **REJECT** | diagnostic must contain `Tuple index` and `out of bounds` |
| BM-608 | `let value = 10; value.0` | **REJECT** | diagnostic must contain `tuple-style access` |
| BM-609 | `let pair = (10,20); pair.0` ; `(Person(age:22), "ok").0.age` | ACCEPT | tuple / named-field-after-tuple access |
| BM-610 | `add(1,2,3)` declared as `add(a,b)` | **REJECT** | error text `Function 'add' expected to receive 2 arguments, instead got 3 arguments`; structured `InvalidFuncallArgLength { func_name, expected_length: 2, received_length: 3 }` (functioncall_invalid_args.rs matches `expected_length: 2`) |
| BM-611 | `add(1)` declared as `add(a,b)` | **REJECT** | structured `InvalidFuncallArgLength { expected_length: 2, received_length: 1 }` |

---

## 4. Diagnostic Anchors (searchable strings)

Phases must keep these byte-for-byte until a phase deliberately announces a
change and updates both this doc and the affected test:

- `moved` — ownership rejections (BM-201..206, 209); actual messages: `Variable 'x' is used after it was moved`, `Cannot move variable 'x' because it is currently borrowed` (ownership.rs asserts `contains("moved")`).
- `Tuple index {index} is out of bounds. The tuple only exposes {length} fields` — tuple arity (BM-607).
- `Type '{ty}' does not support tuple-style access` — non-tuple `x.0` (BM-608).
- `InvalidFuncallArgLength { expected_length, received_length }` → `Function '{name}' expected to receive {expected_length} arguments, instead got {received_length} arguments` — call arity (BM-610/611).
- Mangled specialization names `_<name>_<hash>` with `_`-segment count per
  generic parameter (BM-307) — **I3** from the terms task set.

---

## 5. What Each Future Phase Adds to This Matrix

| Phase (08) | Rows it may flip / add |
|---|---|
| 0 (this doc) | the matrix above |
| 1 Terms | internal-only term snapshots/unifier cases; **no row flips** (must be semantics-preserving) |
| 2 Engine | `copyable(T)`/`method`/`specialize` become queries; unit tests for the three façade consumers; **no user-visible flips** |
| 3.1 Flow, same-semantics port | **no flips** — ownership still rejects the same programs |
| 3.2 Flow, NLL | flips **BM-203** (and any borrow-scoped relaxations); → `ACCEPT`; announced |
| 4 Traits | flips **BM-313/314/315/316/317/318/414/602/605/606** from `ACCEPT` → `REJECT` (validations land); lifts `assert_no_generic_non_functions` (BM-305 generic aliases/styles); adds trait/impl/coherence rows |
| 5 Effects | adds effect/handler/accepted-rejected rows; `@capabilities` (declarations.rs) stops being a stub |
| 6 Typestate | adds per-state accept/reject rows; `transitionable` contract |
| 7 Driver | possibly flips diagnostic routing/dumps (BM-114..116); adds worklist/comptime-synthesized rows |
| 8 Comptime | flips/adopts const-valued-type behavior (the two `unimplemented!` sites: array-len-as-comptime, constant-bound types); adds staging rows |
| 9 Dynamic handlers | optional; adds rows only if the phase is pursued |

---

## 6. Revision Protocol

1. Run `STD_PATH=./lib/std cargo test` on a clean working tree before trusting
   a new matrix edit.
2. Change the **harness markers first** (`// xfail:` / `// xpass:` in
   `examples/generics/`, `examples/enums/`), then this doc, in the same commit
   as the behavior change — never re-baseline a test without a recorded
   rationale in that commit.
3. `ACCEPT` → `REJECT` flips require a marker note (currently all `xpass`
   rows); `REJECT` → `ACCEPT` flips require an `xfail` marker to be removed.