# Project Organization

## Crate layout

Every crate follows the same structure:

```
crates/<name>/
├── Cargo.toml
└── src/
    ├── lib.rs              # Module declarations, re-exports, main struct
    ├── model/              # Data definitions (types, enums, structs)
    │   ├── mod.rs
    │   ├── declarations.rs
    │   ├── expression.rs
    │   └── ...
    ├── modules/            # (optional) Stateful registries
    │   ├── mod.rs
    │   ├── types.rs
    │   └── scopes.rs
    ├── helpers/            # (optional) Simple utility functions
    │   ├── mod.rs
    │   ├── types.rs
    │   └── ...
    └── *.rs                 # Implementation files (flat, no subdirectory)
```

### Rules

1. **`model/`** — Pure data structures only. `#[derive(Debug)]` enums and structs
   with constructors but minimal logic. Model types know nothing about how they
   are built.

2. **`*.rs` at crate root** — All implementation logic lives in flat files
   directly under `src/`. No `implementation/`, `helper/`, or similar
   subdirectories for logic.

3. **`lib.rs`** — Declares all submodules and re-exports public types. The
   crate's main struct is defined here alongside its top-level orchestration
   method (e.g. `generate()`, `check()`). Keep `lib.rs` focused — if the
   orchestration logic grows beyond ~50 lines, move it to its own file
   (e.g. `src/generate.rs`).

4. Each implementation file is an `impl` block on the crate's main struct,
   grouped by concern:
   - `src/declarations.rs` → `impl SlynxHir { fn hoist_stylesheet ... }`
   - `src/expression.rs` → `impl SlynxHir { fn resolve_expr ... }`

5. **`error.rs`** — Every crate that defines error types puts them in
   `src/error.rs`. The type is named `<Crate>Error` (e.g. `HIRError`,
   `TypeError`, `IRError`).

6. **`helpers/`** — Only for simple, stateless utility functions. The kind of
   thing that could be a free function. Examples:
   - `helpers/types.rs` — `int32_type()`, `float32_type()` — one-liner accessors
   - `helpers/names.rs` — `intern_name()`, `retrieve_symbol()` — basic lookups

   Helpers must NOT contain complex lowering, transformation, or analysis
   logic. If a function is more than ~10 lines or calls into model/structural
   code, it belongs in `src/*.rs` as part of the main `impl` block, not in
   `helpers/`.

7. **`modules/`** — Only for stateful registries that accumulate data across
   the entire compilation of a crate. These are long-lived stores (type
   registries, scope stacks, declaration tables). Not for logic.

---

## Helper definition

A helper is a function that:

- Is **simple** (typically 1-5 lines, never more than ~10)
- Is **stateless** (pure lookup, no mutations beyond the registry itself)
- Does **one basic thing** (get a type, intern a name, retrieve a field)

Helper functions are defined on the crate's main struct but grouped in
`src/helpers/` instead of inline to keep the main implementation files
uncluttered. They are typically:

```rust
// helpers/types.rs
impl SlynxHir {
    pub fn int32_type(&self) -> TypeId {
        self.modules.types_module.int_id()
    }
    pub fn float32_type(&self) -> TypeId {
        self.modules.types_module.float_id()
    }
}
```

If a function does any of the following, it does NOT belong in helpers:
- Iterates over declarations or expressions
- Transforms or lowers one representation into another
- Contains match arms on more than 2 variants
- Calls into the checker or IR generator

These belong in the appropriate `src/*.rs` implementation file.

---

## Pipeline overview

```
Source
  │
  ▼ crates/lexer
  │   TokenStream
  │
  ▼ crates/parser
  │   Vec<ASTDeclaration>
  │
  ▼ crates/hir
  │   Vec<HirDeclaration>
  │
  ▼ crates/checker
  │   TypesModule (mutated HIR)
  │
  ▼ crates/monomorphizer
  │   TypesModule (mutated)
  │
  ▼ crates/slynx_ir
  │   SlynxIR
  │
  ▼ src/compilation_context
      .sir file
```

Each crate consumes the previous one's output and produces input for the next.
No crate imports a later stage. `common` is shared by all.

---

## Per-crate breakdown

### `crates/lexer`

```
src/
├── lib.rs         # Lexer struct + tokenize()
├── tokens.rs      # Token, TokenKind
└── error.rs       # LexerError
```

Flat — no model/ directory needed (types are simple enough to live alongside logic).

### `crates/parser`

```
src/
├── lib.rs              # Parser struct + parse_declarations()
├── ast/                # AST data definitions
│   ├── mod.rs
│   ├── expression.rs
│   ├── component.rs
│   └── types.rs
├── expr.rs             # Expression parsing
├── component.rs        # Component parsing
├── conditionals.rs     # If/else parsing
├── functions.rs        # Function parsing
├── objects.rs          # Object parsing
├── statement.rs        # Statement parsing
├── styles.rs           # Stylesheet parsing
├── types.rs            # Type annotation parsing
└── error.rs            # ParseError
```

`ast/` serves as the `model/` directory.

### `crates/hir`

```
src/
├── lib.rs              # SlynxHir struct + generate()
├── model/              # HIR data definitions
│   ├── mod.rs
│   ├── declarations.rs
│   ├── expression.rs
│   ├── statements.rs
│   └── types.rs
├── declarations.rs     # hoist/resolve for declarations
├── expression.rs       # expression resolution
├── statements.rs       # statement resolution
├── components.rs       # specialized component resolution
├── names.rs            # name/type resolution helpers
├── error.rs            # HIRError
├── id.rs               # DeclarationId, TypeId, etc.
├── modules/            # Scope/symbol/type registries
│   ├── mod.rs
│   ├── declarations.rs
│   ├── scopes.rs
│   ├── symbols.rs
│   └── types.rs
└── helpers/            # Utility functions
    ├── mod.rs
    ├── expressions.rs
    ├── names.rs
    └── types.rs
```

### `crates/checker`

```
src/
├── lib.rs         # TypeChecker struct + check()
├── decl.rs        # Declaration type-checking
├── defaults.rs    # Fallback type assignment
├── expr.rs        # Expression type-checking
├── statement.rs   # Statement type-checking
├── styles.rs      # Stylesheet type-checking
└── error.rs       # TypeError
```

Flat — no model/ directory needed (types are re-exported from `hir`).

### `crates/monomorphizer`

```
src/
└── lib.rs         # Monomorphizer struct + resolve()
```

### `crates/slynx_ir`

```
src/
├── lib.rs         # Module declarations, re-exports
├── ir/            # IR data definitions
│   ├── mod.rs
│   └── model/
│       ├── mod.rs
│       ├── context.rs
│       ├── components.rs
│       ├── instruction.rs
│       ├── label.rs
│       ├── ptr.rs
│       ├── value.rs
│       └── styles.rs
├── api.rs         # Public IR accessors
├── components.rs  # Component lowering
├── contexts.rs    # Context management
├── instructions.rs# Instruction builders
├── temp.rs        # Transient state (TempIRData)
├── types/         # IR type system
│   ├── mod.rs
│   ├── irtype.rs
│   ├── components.rs
│   ├── functions.rs
│   ├── structs.rs
│   └── tuple.rs
├── cfg/           # Control-flow graph
│   └── mod.rs
├── visualize/     # IR pretty-printer
│   ├── mod.rs
│   └── formatter.rs
└── error.rs       # IRError
```

### `crates/common`

```
src/
├── lib.rs         # Span, Operator
└── symbols.rs     # SymbolsModule, SymbolPointer
```

Flat — shared primitives used by all crates.

---

## Patterns per role

| Role | Where it lives | Example |
|---|---|---|
| Data types | `src/model/*.rs` | `HirDeclaration`, `StyleProperty`, `Instruction` |
| Main struct | `src/lib.rs` | `SlynxHir`, `TypeChecker`, `SlynxIR` |
| Orchestration | `src/lib.rs` (or `src/generate.rs` if large) | `generate()`, `check()` |
| Phase 1 logic | `src/<name>.rs` | `hoist_stylesheet()`, `resolve_specialized()` |
| Phase 2 logic | `src/<name>.rs` | `default_stylesheet()`, `lower_stylesheet()` |
| Error types | `src/error.rs` | `HIRError`, `TypeError`, `IRError` |
| ID types | `src/id.rs` | `DeclarationId`, `TypeId` |
| Registries | `src/modules/*.rs` | `TypesModule`, `ScopeModule` |
| Simple utilities | `src/helpers/*.rs` | `int32_type()`, `float32_type()` |

## Naming conventions

See [`name-convention.md`](name-convention.md) for the naming conventions used
across phases.

---

## Status vs. the target pattern

| Crate | Follows pattern? | Issues |
|---|---|---|
| common | ✅ | — |
| lexer | ✅ | — |
| parser | ✅ | — |
| hir | ⚠️ | `implementation/` folder — logic should be in `src/*.rs` |
| checker | ✅ | — |
| monomorphizer | ✅ | (single file is fine) |
| slynx_ir | ❌ | `ir/model/` should be `src/model/`. `ir/helper/` should be flat `src/*.rs`. The `ir/` wrapper directory is redundant because the crate is already `slynx_ir`. |

---

## Rust project organization assessment

The project generally follows standard Rust conventions but has several areas
that deviate from idiomatic practice:

### What follows convention ✅

- **Workspace layout**: Each major subsystem is a separate crate with its own
  `Cargo.toml`. This is the standard approach for multi-crate Rust projects.
- **`lib.rs` as crate root**: Each crate exposes its public API through
  `lib.rs`. Correct.
- **Error types in dedicated `error.rs`**: Consistent across all crates. Good.
- **File-per-concept naming**: Files are named after their primary type
  (`lexer.rs`, `parser.rs`, `expression.rs`). This is the standard convention.
- **`tests/` directory at workspace root**: Integration tests live in the
  top-level `tests/` directory. Standard.

### What could be improved ⚠️

#### 1. `slynx_ir` has redundant nesting

```
src/ir/model/       → should be  src/model/
src/ir/helper/      → should be  src/*.rs (flat)
src/ir/components.rs → should be  src/components.rs
```

The crate is named `slynx_ir`. Having an `ir/` subdirectory inside it is
redundant — the entire crate IS the IR. Every file is already "IR" code.
This adds 1-2 unnecessary directory levels to every import path.

**Fix**: Flatten `ir/model/` → `model/`, `ir/helper/` → `*.rs`,
`ir/*.rs` → `src/*.rs`. Remove the `ir/` wrapper.

#### 2. `lib.rs` in checker and HIR is too large

Checker's `lib.rs` is 423 lines and contains the `TypeChecker` struct,
the top-level `check()` orchestration, AND the entire unification engine
(`unify()`, `unify_with_ref()`, `recursive_ty()`).

HIR's `lib.rs` is 445 lines and contains the `SlynxHir` struct AND the
entire `generate()` two-pass orchestration.

Standard Rust convention is for `lib.rs` to be lean module declarations
and re-exports, with logic in named files.

**Fix**: Move unification from checker's `lib.rs` to `src/unify.rs`.
Move HIR's `generate()` body to `src/generate.rs`.

#### 3. `slynx_ir/src/ir/model/` + `slynx_ir/src/types/` — two type systems

The IR crate has two directories defining types:
- `src/ir/model/` — IR instruction model (Context, Label, Value, Instruction, StyleProperty)
- `src/types/` — The IR type system (IRType, IRTypes, structs, functions, components)

These serve different purposes, but having both at different nesting levels
is confusing. They should either be merged into `src/model/` or both live
at the same level with clear names.

#### 4. No `pub use` re-export pattern for model types

In idiomatic Rust, a crate's public API is typically re-exported through
`lib.rs` so consumers write `slynx_hir::HirDeclaration` instead of
`slynx_hir::model::HirDeclaration`. Some crates do this (HIR re-exports
model, error, id) but the pattern is not consistently applied.

#### 5. `helpers/` in HIR is inconsistently used

`helpers/expressions.rs`, `helpers/names.rs`, and `helpers/types.rs` contain
one-liner accessors. This is correct per the helper definition above. However,
`helpers/names.rs` also contains `retrieve_information_of_type()` which is
~15 lines and does non-trivial lookup logic — this should arguably be in
`src/names.rs` alongside the other name resolution functions.

#### 6. Inline tests are sparse

Rust convention is to include `#[cfg(test)] mod tests { ... }` at the bottom
of implementation files. Most files in this project lack inline tests, relying
instead on integration tests in the top-level `tests/` directory. While
integration tests are valuable, unit tests inline would improve coverage and
document edge cases.

### Summary of recommended changes

| Change | Crate | Effort |
|---|---|---|
| Flatten `ir/` → remove wrapper | slynx_ir | 1hr |
| Move `ir/helper/` logic to flat `*.rs` | slynx_ir | 30min |
| Move `unify()` to `src/unify.rs` | checker | 15min |
| Move `generate()` body to `src/generate.rs` | hir | 15min |
| Add `pub use` re-exports to lib.rs | all | 10min each |
| Add inline unit tests | all | ongoing |
