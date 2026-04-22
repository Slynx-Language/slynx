# Component Reactivity Model

This document captures the current design direction for component reactivity in
Slynx.

It exists to close the semantic contract of ownership, binding, and upward
updates **before** parser, HIR, graph generation, and IR lowering start
depending on assumptions that may later need to be undone.

## Status

This is a design/specification document.

It does **not** mean reactivity is already fully implemented on the current
`main` branch.

This document is scoped to the **first reactivity implementation target**,
currently understood as the ownership model intended for `v0.0.1`-level work.

The goal of this document is to record:

- what direction is already recommended for the first implementation;
- what rules keep the model predictable for contributors and backends;
- what still needs explicit approval before syntax and lowering are locked.

## Goal

The reactivity model should make it obvious:

- who owns a mutable value;
- which values are only inputs;
- how child components can request updates they do not own;
- where style and animation fit without becoming hidden mutation systems.

The main objective is predictability. The first implementation should avoid
reactive behavior that looks convenient at the syntax level but becomes unclear
once lowering, debugging, and backend execution are involved.

## High-Level Direction

The recommended direction for the first implementation is:

- every mutable reactive value has **one owning component**;
- reactive inputs flow **from parent to child**;
- a child may **read** a bound input, but should not directly mutate it;
- if a child wants to affect non-owned data, it should emit an
  **event/command** upward;
- style should be modeled as **derived data**, not as a second hidden state
  system;
- animation should be modeled as an **effect/transition layer**, not as just
  another data-binding edge.

This intentionally prefers explicit ownership over compile-time aliasing tricks.

## Core Concepts

### 1. Owned State

An owned value belongs to one component and is mutated only by that component.

Examples:

- internal state declared and updated inside a component;
- public component data that is still owned by that component once the instance
  exists.

The important part is semantic ownership, not whether the syntax spells it as
`prop`, `state`, or something else in the future.

### 2. Bound Input

A bound input is a value that flows from an owner into another component.

The receiving component may use that value in:

- child props;
- computed expressions;
- style derivations;
- control flow;
- event payload construction.

For the first implementation, the receiving component should treat that value as
**read-only**.

### 3. Derived Value

A derived value is any pure computation based on owned state and/or bound
inputs.

Examples:

- `count * 2`
- `isSelected && isEnabled`
- `ButtonStyle(primary, hovered)`

Derived values are the natural input to the reactive dependency graph.

### 4. Event / Command Output

When a component wants to request a change to data it does not own, it should
emit an event or command upward.

Conceptually:

- child requests a change;
- owner receives the request;
- owner decides whether and how to mutate its own state.

This keeps mutation localized to the owner and avoids hidden aliasing between
component boundaries.

## Recommended Rules For The First Implementation (`v0.0.1` Scope)

To keep the first implementation small and predictable, this document
recommends the following defaults:

1. Every mutable value must have **exactly one owner**.
2. Parent-to-child reactive inputs should be treated as **readable**, not
   directly writable, inside the child.
3. Child-to-parent updates should be represented as **events/commands**, not as
   direct mutation of the parent's storage.
4. The first implementation should **not** rewrite child mutation of non-owned
   data into direct parent mutation "behind the scenes" just because that is
   possible at compile time.
5. Style should be expressed as a **derived result** of props/state, not as an
   independent mutation channel with separate ownership rules.
6. Animation should be triggered by **state changes or events**, and should stay
   conceptually separate from the pure value-dependency graph.

These rules are meant to reduce ambiguity and keep later middleend/IR work
consistent with what contributors expect from the source language.

## Preferred Direction vs Rejected Shortcut

### Direction To Avoid For The First Version

The first implementation should avoid a model where a child mutates a bound
value and the compiler silently rewrites that mutation into a write on the
parent's owned storage.

Even if this can be optimized well, it creates problems for:

- debugging ownership;
- understanding who is allowed to mutate what;
- reasoning about multiple children targeting the same source;
- layering style and animation semantics on top later.

### Preferred Direction

For the first implementation, the safer model is:

1. owner data flows down;
2. child derives from it;
3. child emits requests up;
4. owner handles the request and mutates its own state.

Conceptually:

```slynx
component Parent {
  pub prop count = 0;

  func handleChild(event: ChildEvent) {
    switch(event) {
      case .Increment(n): count += n
    }
  }

  Child {
    count: count,
    on_event: event -> handleChild(event),
  }
}

enum ChildEvent {
  Increment(int)
}

component Child {
  pub bind count: int;

  Button {
    on_click: _ -> emit Increment(1)
  }
}
```

This example should be read as **semantic direction**, not as finalized syntax.

## Style And State

Style should not introduce a second hidden ownership model.

For the first implementation, the safest rule is:

- style values are computed from owned state and/or bound inputs;
- style evaluation can later reuse the same dependency-discovery machinery as
  other derived values;
- style itself should not imply separate writable state unless the team
  explicitly designs such a feature later.

This keeps style understandable as "visual data derived from component data"
instead of "another reactive subsystem with its own mutation rules".

## Animation And Effects

Animation should be treated differently from plain value derivation.

Why:

- animation is often temporal;
- animation may need backend/runtime policies;
- animation may depend on transitions, not just current values.

For the first implementation, a good default is:

- state changes and events may trigger animations;
- animation lowering should be a separate effect/transition concern;
- the pure dependency graph should stay focused on values and deterministic
  derived updates.

This does **not** forbid future animation syntax. It only avoids forcing the
first reactivity graph to also become a complete animation scheduler.

## Relationship To Graph Generation And IR

This ownership model should guide the later middleend work:

- the reactive graph should model **derived data flow** and **downward
  propagation**;
- upward requests should remain explicit as **events/commands**;
- `@bind`-like operations should represent value propagation, not hidden writes
  into someone else's owned state;
- `@emit`-like operations are the natural place to represent outward update
  requests;
- `@rerender` remains a consequence of owner-visible state changes, not proof
  that ownership boundaries disappeared.

In other words: graph generation should stay about dependencies, while events
carry cross-boundary mutation requests.

## What This Document Deliberately Does Not Lock Yet

Some parts still need a final decision and should stay open until the team wants
to implement them:

### 1. Final Surface Syntax

This document does **not** finalize whether the language should spell concepts
as:

- `prop`
- `state`
- `bind`
- `emit`
- `command`
- `event`

It only locks the semantic separation they should represent.

### 2. Event Handler Placement

This document does not finalize whether handlers should always live:

- inside the component body;
- in an extension;
- in a separate helper construct.

### 3. Runtime Transport Strategy

This document does not require a queue, closure, callback object, or any other
specific runtime implementation strategy.

That remains a backend/runtime concern as long as the semantic contract stays
the same.

### 4. Final Style Syntax

This document does not lock the final syntax sugar for styles, style shorthands,
or style composition.

### 5. Final Animation Syntax

This document does not lock an animation DSL, transition syntax, or backend
policy for interruption, replay, or timing.

## Non-Goals For The First Version (`v0.0.1` Scope)

The first ownership-focused reactivity implementation does **not** need to
solve:

- backend scheduling policy;
- async delivery guarantees for events;
- batching/coalescing strategy;
- animation engine semantics;
- finalized style DSL;
- cross-component mutation shortcuts that hide ownership.

Those can be layered later once the ownership contract is stable.

## Suggested Implementation Order

The safest order is:

1. finalize the ownership and event contract;
2. define frontend/type-checking rules for owned vs bound values;
3. generate the dependency graph only for derived/downward updates;
4. lower upward requests through explicit event/command semantics;
5. only then design style/animation syntax on top of that base.

This keeps the first reactivity work small, predictable, and easier to carry
forward into HIR, graph generation, and IR.
