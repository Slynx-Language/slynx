# Landing Content Inventory

This document is a grounded inventory of what the `Slynx-Language/slynx` repository can already
support on the landing page and documentation site today.

The goal is to help the team move content into `slynx-web` without promising features that are not
implemented on `main`.

## How To Use This Document

Each topic below is labeled according to the current state of the repository:

- `Implemented`: backed by code or repository workflow on `main`
- `Spec / Design`: documented direction, but not fully implemented on `main`
- `Missing`: not ready to publish as if it already exists

When in doubt, prefer the stricter label.

## Source Of Truth In This Repository

These files are the main factual references for landing-page content:

- [README.md](../README.md): project overview, workspace layout, current status, and current
  release/tag status
- [docs/language-surface.md](language-surface.md): grounded overview of the current syntax that
  `main` already supports
- [docs/first-slynx-file.md](first-slynx-file.md): short onboarding guide with a real `.slynx`
  example
- [CONTRIBUTING.md](../CONTRIBUTING.md): contribution workflow and validation expectations
- [GOVERNANCE.md](../GOVERNANCE.md): maintainership and decision structure
- [RELEASING.md](../RELEASING.md): release/tag process
- [CHANGELOG.md](../CHANGELOG.md): release history
- [middleend/README.md](../middleend/README.md): middleend and IR design reference
- [docs/reactivity-model.md](reactivity-model.md): ownership-first reactivity model spec
- [middleend/docs/reactive-graph-generation.md](../middleend/docs/reactive-graph-generation.md):
  reactive graph generation spec
- [docs/component-slots.md](component-slots.md): slot model spec for the first implementation

## Content That Is Ready For The Landing Page

### 1. Project Positioning

Status: `Implemented`

Safe claims today:

- Slynx is an experimental UI language project
- the current repository is library-first
- the current workspace is focused on the frontend and middleend of the language
- `main` can lex, parse, build HIR, run type checking, resolve the current alias surface, and lower to the current `SlynxIR`
- the root crate can write the default `.sir` output and can expose `.hir` / `.ir` dumps through `SlynxContext::build_stages()`

Evidence:

- [README.md](../README.md)
- [src/lib.rs](../src/lib.rs)
- [src/context.rs](../src/context.rs)

Claims to avoid:

- that `main` ships an official CLI binary
- that `main` ships an official backend compiler
- that the IR is already stable as a public contract
- that the language is production-ready

### 2. Current Workspace Architecture

Status: `Implemented`

Safe content:

- `common/`: shared AST and common language structures
- `frontend/`: lexer, parser, HIR generation, and type checking
- `middleend/`: current `SlynxIR` and IR/lowering work
- root crate: compile helpers, context, and error presentation

Evidence:

- [README.md](../README.md)
- [common/](../common)
- [frontend/](../frontend)
- [middleend/](../middleend)
- [src/](../src)

### 3. Getting Started As A Contributor

Status: `Implemented`

Safe content:

- prerequisites: Rust stable + Cargo
- standard validation flow:
  - `cargo build`
  - `cargo test`
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
- project workflow and contribution expectations

Evidence:

- [README.md](../README.md)
- [CONTRIBUTING.md](../CONTRIBUTING.md)
- [docs/first-slynx-file.md](first-slynx-file.md)

### 4. Current Language Surface

Status: `Implemented`

Safe content:

- top-level `object`, `component`, and `func` declarations
- top-level `alias` declarations
- block-bodied and arrow-bodied functions
- `let` / `let mut`
- assignment
- `while`
- function calls, including zero-argument calls
- tuple types and tuple literals
- object expressions
- component expressions
- field access
- arithmetic/comparison/logical/bitwise binary expressions
- `if` expressions

Evidence:

- [docs/language-surface.md](language-surface.md)
- [frontend/src/parser](../frontend/src/parser)
- [frontend/tests/parser.rs](../frontend/tests/parser.rs)

### 5. Public Library Entry Points

Status: `Implemented`, but narrow

Safe content:

- `slynx::compile_code(...)`
- `slynx::compile_to_ir(...)`
- `SlynxContext`
- `SlynxContext::build_stages()`
- `CompilationStages`
- `.hir` / `.ir` dump writing through the library API
- public module re-exports for `checker`, `hir`, `lexer`, and `parser`

Evidence:

- [src/lib.rs](../src/lib.rs)
- [src/context.rs](../src/context.rs)

Important limitation:

- there is not yet a polished, versioned API reference for the whole project
- the dump outputs are still debug-style textual artifacts, not a stable versioned public format
- landing content can mention the public entry points, but should not pretend a complete API
  reference or polished CLI workflow already exists

### 6. Governance / Community / Releases

Status: `Implemented`

Safe content:

- maintainership/governance structure
- contribution flow
- public tag `v0.0.1`
- release process by tags

Evidence:

- [GOVERNANCE.md](../GOVERNANCE.md)
- [CONTRIBUTING.md](../CONTRIBUTING.md)
- [RELEASING.md](../RELEASING.md)
- [CHANGELOG.md](../CHANGELOG.md)
- [Git tags](https://github.com/Slynx-Language/slynx/tags)

Important limitation:

- the repository has a public tag, but there is still no published GitHub Release page at the time
  reflected in the current docs

## Content That Can Be Published Only As Spec / Design

### 1. Middleend / IR Design

Status: `Spec / Design`

Safe framing:

- the repository already contains a design/specification reference for the middleend
- the design direction is broader than what `main` emits today

Evidence:

- [middleend/README.md](../middleend/README.md)

Do not present as fully implemented:

- the full IR surface described in `middleend/README.md`
- every operation/example in that document as if it already exists in the codebase

### 2. Reactivity Ownership Model

Status: `Spec / Design`

Safe framing:

- the repository contains a design proposal for ownership-first component
  reactivity;
- it separates owned state, bound inputs, upward events/commands, style
  derivation, and animation concerns;
- it is intended to guide frontend, graph, and IR work before the runtime model
  is finalized.

Evidence:

- [docs/reactivity-model.md](reactivity-model.md)

Do not present as implemented:

- finished component reactivity on `main`;
- finalized event syntax;
- finalized style/animation language features.

### 3. Reactive Graph Generation

Status: `Spec / Design`

Safe framing:

- there is a documented proposal for the graph-generation stage
- it defines pipeline position, inputs, outputs, node/edge modeling, and non-goals

Evidence:

- [middleend/docs/reactive-graph-generation.md](../middleend/docs/reactive-graph-generation.md)

Do not present as implemented:

- reactive graph lowering as a finished compiler pass
- final graph-driven IR generation on `main`

### 4. Component Slots

Status: `Spec / Design`

Safe framing:

- the repository has a slot-model spec for the first implementation
- the document defines the default-slot plus named-slot direction and the intended `v0.0.1`
  implementation scope

Evidence:

- [docs/component-slots.md](component-slots.md)

Do not present as implemented:

- parser support for slots
- HIR support for slots
- IR lowering for slots

## Content That Should Not Be Marketed As Available Yet

Status: `Missing`

These topics should not be published as current product features:

- official CLI binary on `main`
- official backend compiler on `main`
- polished CLI flags/workflow for dump generation on `main`
- stable versioned textual `.hir` / `.ir` dump contracts
- implemented component slots
- implemented reactive graph lowering
- stable generics / monomorphization pipeline
- production-ready component lowering in the IR

## Suggested Landing Structure

This is a safe structure for `slynx-web` based on what exists today.

### Home

Use only implemented or clearly qualified messaging:

- experimental UI language
- frontend + middleend workspace
- library-first current state
- current contributor workflow

### Docs

Good candidates to turn into landing/docs pages now:

- What is Slynx?
- Current workspace layout
- Getting started for contributors
- Current compile/lower pipeline
- Governance and contribution flow

### API Reference

Start small and honest:

- root helpers: `compile_code`, `compile_to_ir`, `SlynxContext`, `build_stages`
- note that broader API reference is still incomplete

### Features

Only present features with strict wording:

- lexer + parser
- HIR generation
- type checking
- lowering to current `SlynxIR`
- aliases, tuple literals/types, and `while` in the current frontend surface
- library-side `.hir` / `.ir` / `.sir` dump generation
- CI + release/tag workflow

Avoid broad claims like:

- complete compiler toolchain
- stable backend architecture
- production-ready IR

### Use Cases

Safe current use cases:

- studying the language frontend pipeline
- experimenting with parser/HIR/type-checker behavior
- contributing to language design and compiler passes
- validating language ideas through specs and regression tests

## Immediate Next Steps For The Landing

If the team wants to move quickly without inventing content, this order is the safest:

1. move the implemented project overview from [README.md](../README.md) into `slynx-web`
2. expose contributor onboarding from [CONTRIBUTING.md](../CONTRIBUTING.md)
3. add a small API-reference page for the current root helpers
4. add a clearly labeled "Design Docs" section for:
   - [middleend/README.md](../middleend/README.md)
   - [docs/reactivity-model.md](reactivity-model.md)
   - [middleend/docs/reactive-graph-generation.md](../middleend/docs/reactive-graph-generation.md)
   - [docs/component-slots.md](component-slots.md)
5. leave unfinished or speculative features out of the marketing copy until they land on `main`
