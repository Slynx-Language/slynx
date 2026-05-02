# Reactive Graph Generation

This document specifies the stage that builds Slynx's reactive dependency graph
for a component. It intentionally stops at graph generation.
Linearization and IR lowering are left as separate phases.

## Goal

The graph generation stage extracts reactive dependencies from a typed component
definition and produces a generic graph representation that later phases can:

- validate for cycles;
- linearize deterministically;
- lower into IR UI operations such as `@bind`, `@emit`, and `@rerender`.

The graph itself should stay generic enough to be reused by future compiler
features and tooling. The IR should only consume the graph output; it should not
own the dependency discovery logic.

## Placement In The Pipeline

For `v0.0.1`, the intended order is:

1. parse;
2. type-check;
3. fill omitted component props with their default values;
4. generate the reactive dependency graph for each component;
5. linearize the graph;
6. lower the linearized operations into IR.

If generics/monomorphization are added later, the ideal position becomes:

1. parse;
2. type-check;
3. monomorphize;
4. fill omitted component props with defaults;
5. generate the reactive dependency graph;
6. linearize the graph;
7. lower into IR.

This keeps graph generation independent from generic syntax while still leaving
the door open for monomorphized component specializations later.

## Inputs

Graph generation consumes a single typed component declaration after prop types
and child expressions have already been validated.

The stage needs:

- the component declaration itself;
- the component prop list, including default values when present;
- the typed expressions used by child properties and reactive updates;
- the field index/type metadata for child components and specialized nodes.

## Outputs

The output of this stage is a graph definition for one component.

Conceptually:

```rust
struct ReactiveGraph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}
```

The exact Rust API can change, but the semantics should stay the same:

- `nodes` represent dependency sources or sinks;
- `edges` represent updates that happen when a source changes;
- edge metadata carries the transformation path that should be applied before
  writing into the target.

## Node Model

For `v0.0.1`, graph generation only needs a small set of node kinds:

- component prop node;
- internal computed value node;
- child field node;
- specialized child field node.

Suggested conceptual model:

```rust
struct PropId(usize);
struct ComputedValueId(usize);
struct ChildId(usize);
struct FieldId(usize);

enum GraphNodeKind {
    ComponentProp { prop_id: PropId },
    ComputedValue { id: ComputedValueId },
    ChildField { child_id: ChildId, field_id: FieldId },
    SpecializedField { child_id: ChildId, field_id: FieldId },
}
```

The important part is not the exact enum name, but that nodes are stable,
addressable, and backend-agnostic.

Even if the underlying storage ends up being backed by `usize`, the exposed API
should prefer semantic wrapper types over raw integers so each identifier
retains its meaning across future refactors.

## Edge Model

An edge means:

> when the source node changes, compute the target value and update the target
> node.

Each edge should carry:

- `source`;
- `target`;
- zero or more modifiers applied in order.

Suggested conceptual model:

```rust
struct GraphEdge {
    source: NodeId,
    target: NodeId,
    modifiers: Vec<ModifierRef>,
}
```

Examples:

- `count -> field #t0, 0`
  - source: component prop `count`
  - target: field `0` of child `#t0`
  - modifiers: none

- `count |> f -> field #t1, 0`
  - source: component prop `count`
  - target: field `0` of child `#t1`
  - modifiers: `[f]`

## Modifier Semantics

Modifiers represent a pure transformation path that consumes the current source
value and returns the value that must be written into the target.

For `v0.0.1`, a modifier chain should:

- preserve the declared order;
- be resolved from already type-checked expressions;
- avoid embedding backend details.

Examples:

- `count` -> no modifiers;
- `count |> f` -> one modifier;
- `count |> f |> g` -> two modifiers, applied left-to-right.

This lets later phases lower the same edge into:

- direct assignment when `modifiers.is_empty()`;
- one or more calls before the final write when modifiers exist.

## Generation Rules

### 1. Defaults First

If a prop is omitted in component construction, graph generation should behave
as if the default value had already been made explicit by an earlier phase.

This keeps graph generation free from "optional prop input" branching.

### 2. One Node Per Stable Dependency Endpoint

Nodes should represent stable dependency endpoints, not arbitrary syntax trees.

Good:

- `Counter.count`
- `child #t0 field 0`

Bad:

- raw AST fragments
- backend-specific handles

### 3. One Edge Per Reactive Update

Each independently triggerable update should become its own edge.

If two child fields both depend on `count`, they become two edges.
If one target depends on `count |> f`, that transformation belongs to that edge.

### 4. No Lowering During Generation

This stage should not emit IR instructions and should not decide label/block
layout. It only builds the dependency graph.

### 5. Cycle Detection Is Mandatory

The generated graph must be validated as acyclic before linearization.

If a cycle is found, graph generation should fail with a middleend error that
identifies the component and the participating dependency chain when possible.

## Counter Example

Given:

```slynx
func f(n: int): int {
  n * 2
}

component Counter {
  pub prop count = 0;

  Text {
    text: count
  }

  Text {
    text: f(count)
  }
}
```

The generated graph should be logically equivalent to:

```text
nodes:
  n0 = prop(count)
  n1 = child(#t0).field(0)
  n2 = child(#t1).field(0)

edges:
  n0 -> n1 []
  n0 -> n2 [f]
```

Linearization is not part of this document, but a later phase can consume this
graph and emit:

- direct bind for `n0 -> n1`;
- bind with one modifier call for `n0 -> n2`.

## Non-Goals For v0.0.1

The first version of this stage does not need to solve:

- generic-aware graph specialization;
- graph serialization format;
- backend scheduling strategy;
- incremental recomputation caching;
- conditional exhaustiveness over every branch mutation case;
- child lifecycle policies such as hide/reveal cache ownership.

Those can be layered on top once the graph generation contract is stable.

## Why This Split Helps

Keeping graph generation separate from linearization and IR lowering gives us:

- a smaller surface to reason about;
- easier testing;
- a reusable dependency model for future passes;
- less coupling between frontend semantics and backend/runtime details.

That separation is especially useful while the IR is still evolving.
