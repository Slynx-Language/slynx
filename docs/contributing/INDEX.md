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
| [`styles.md`](styles.md) | Stylesheet system: pipeline, how it works, how to add new style properties |

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
