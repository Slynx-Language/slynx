# Contributor Docs

The files in this folder document specific parts of the codebase:
how they work, how to extend them, and where to modify behavior.

## Goals

These docs are intended to help contributors:

- understand architecture decisions
- locate implementation details quickly
- extend existing systems safely
- avoid regressions and duplicated logic

## Organization

Each document should focus on a single subsystem or feature.

Recommended structure for new docs:

- Overview
- Core concepts
- Relevant files/modules
- Data flow / execution flow
- Extension points
- Common pitfalls
- Examples

When documenting specific functionality (such as pipelines, subsystems,
or individual functions), always mention where the implementation is
located in the codebase.

Prefer including:

- file paths
- module names
- important entry points
- relevant functions/types

This helps contributors quickly navigate from documentation to code.


## Existing Docs

| File | Description |
|------|-------------|
| [`project-organization.md`](project-organization.md) | Crate layout, pipeline overview, and per-crate conventions |
| [`styles.md`](styles.md) | Stylesheet system: pipeline, how it works, how to add new style properties |
| [`generics-implementation.md`](generics-implementation.md) | How generics work: parser, AST, HIR, monomorphizer, and known limits |
| [`generics-inference.md`](generics-inference.md) | Current state of generic type inference (explicit type arguments only) |
| [`intrinsic-types-codegen.md`](intrinsic-types-codegen.md) | How `@intrinsic`-tagged objects are lowered by the codegen |
| [`boostraping-components.md`](boostraping-components.md) | Bootstrapped components contract: context, slots, lifecycle |
| [`string-interning.md`](string-interning.md) | Symbol interning strategy (`SymbolPointer`) |
| [`main-goal.md`](main-goal.md) | Main goal of the language and its remaining features |

## Conventions

- Prefer practical explanations over theory
- Link directly to source files when relevant
- Keep examples minimal and runnable
- Update docs whenever behavior changes

## When To Add Documentation

Add or update contributor docs when:

- introducing a new subsystem
- changing architecture significantly
- adding non-obvious behavior
- fixing bugs caused by implicit assumptions
- adding extension APIs or customization points
