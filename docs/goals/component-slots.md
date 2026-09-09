# Component Slots

This document captures the current design direction for component slots in
Slynx.

It exists to close the contract of slots **before** parser, HIR, and IR work
start depending on assumptions that may later need to be undone.

## Status

This is a design/specification document.

It does **not** mean slot syntax is already implemented on the current `main`
branch.

This document is scoped to the **first slot implementation target**, currently
understood as the slot model intended for `v0.0.1`-level work.

The goal of this document is to record:

- what direction is already agreed;
- what rules are recommended for the `v0.0.1` / first implementation;
- what still needs explicit approval before code changes start.

## Goal

Slots are meant to give component children an explicit structural model instead
of treating every child as "just another nested component by position".

That helps with:

- clearer composition semantics;
- more stable frontend/HIR representation;
- easier child representation in the future IR;
- less chance of the component model being implemented in a shape that later
  needs to be rewritten.

## High-Level Direction Already Agreed

The current direction discussed by the maintainers is:

- components should be able to declare slot positions;
- a slot may be unnamed, in which case it acts as the **default slot**;
- a slot may be named;
- the slot model should stay generic enough to be useful later in the pipeline,
  especially when components are eventually represented in IR.

The intent is that future lowering stages work with **slot identities** instead
of relying only on raw child order.

## Declaration Syntax

### Default Slot

The default slot is declared as:

```slynx
Slot {}
```

Example:

```slynx
component Bordered {
  pub prop color: color;

  Div {
    style: Bordered(color, 1px, 12px),
    Slot {}
  }
}
```

### Named Slot

A named slot is declared by prefixing the slot with an identifier:

```slynx
header: Slot {}
```

Example:

```slynx
component Bordered {
  pub prop color: color;

  Div {
    style: Bordered(color, 1px, 12px),
    header: Slot {}
  }

  footer: Slot {}
}
```

## Intended Semantics

### Default Slot

The default slot exists to keep the common composition case simple.

Conceptually, it is the place that receives the component's primary unnamed
child content.

### Named Slots

Named slots exist for components that expose more than one composition region.

They give those regions stable identities and make the component contract more
explicit than relying on position alone.

Examples of the kind of distinction named slots are meant to support:

- main content vs footer;
- header vs body;
- leading icon vs trailing icon;
- primary child region vs auxiliary child region.

## Representation Goals For Future Phases

Even before IR lowering exists for this, the slot model should aim for these
properties:

- every slot has a stable identity inside its owning component;
- the default slot is still addressable as a stable identity, even though it is
  unnamed at the syntax level;
- named slots keep their declared names;
- children are associated with slots, not treated as purely anonymous position
  entries.

This is the main reason the slot contract should be specified early.

## Recommended Rules For The First Implementation (`v0.0.1` Scope)

To keep the first implementation small and predictable, this document recommends
the following defaults:

1. A component may declare **at most one default slot**.
2. Named slot names must be **unique within the component**.
3. Slot declarations should be preserved as explicit slot declarations in the
   frontend/HIR instead of being erased into generic child nodes too early.
4. Slot identity should remain independent from future generics work.

These recommendations are meant to reduce ambiguity and keep the model easy to
carry forward into HIR and IR later.

## What This Document Deliberately Does Not Lock Yet

Some parts still need a final decision and should stay open until the team wants
to implement them:

### 1. Syntax For Supplying Named Slot Content

This document records the syntax for **declaring** named slots, but it does not
yet lock the final syntax for **providing** children to those named slots from a
component use site.

That should be decided before parser work starts.

### 2. Behavior When Unnamed Child Content Exists But No Default Slot Exists

This case should be made explicit.

Reasonable choices include:

- reject it as an error;
- allow it only under some compatibility rule;
- lower it through a specific implicit slot behavior.

This document does not pick one yet.

### 3. Whether Slot Order Carries Semantic Meaning Beyond Stability

Declaration order is useful for stable identity and diagnostics, but whether
order should have extra language meaning beyond that is still open.

### 4. Whether Future Slot Variants Need More Metadata

For example:

- required vs optional slot;
- slot visibility;
- slot type/category constraints.

These are explicitly out of scope for the first version.

## Non-Goals For The First Version (`v0.0.1` Scope)

The first slot implementation does **not** need to solve:

- IR representation of component instances;
- reactivity over slots;
- slot serialization format;
- generic-aware slot specialization;
- const-generic-driven slot behavior;
- runtime lifecycle policies for slotted children.

Those can be layered later once the basic contract is stable.

## Suggested Implementation Order

The safest order is:

1. finalize the slot contract;
2. add parser/AST support only;
3. preserve slot information in HIR;
4. only then design how the middleend/IR consumes slot metadata.

This keeps the first PRs small and avoids forcing IR design to guess at syntax
and semantics that the frontend has not finalized yet.
