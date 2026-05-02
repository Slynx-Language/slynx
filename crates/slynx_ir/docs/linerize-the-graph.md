# This document specifies the linearization phase of the reactive graph in Slynx.

After generating the dependency graph (see Reactive Graph Generation), it is necessary to transform this graph into a **linear sequence** of execution that respects the dependencies between the nodes.

This step is essential to allow later phases (**such as lowering to IR
or runtime execution**) to process reactive updates in a deterministic and efficient manner.

We will use _Kahn's Algorithm_, which is a _topological sorting_ using _BFS_.

The goal of linearization is to produce an execution order where:

- each node appears before all nodes that depend on it;

- no dependencies are violated during execution;

- the result is deterministic (when possible);

- the processing cost is linear with respect to the size of the graph.

The algorithm receives a _directed acyclic graph (DAG)_
and returns a linear sequence where each node appears
before the nodes that depend on it.

Given a graph:

- Calculate the in-degree of each node;

- Initialize a queue with all nodes with in-degree = 0;

- Iteratively:

  - Remove a node from the queue;

  - Add that node to the linear output;

  - Reduce the in-degree of its neighbors;

  - Any neighbor that reaches 0 is added to the queue;

- If all nodes are processed → success;

- Otherwise → a cycle exists.

## Dependence comes first.

### If there is an edge
```text
  A -> B
```


### So, in linear order

```text
[A,B]
```

## Independence allows for multiple orders.

### If two nodes have no dependency on each other:

```text
  A -> B
  A -> C
```


### So both are valid:
#### 1)
```text
  [A, B, C]
```

#### 2)
```text
  [A, C, B]
```

## Order does not guarantee total stability

The algorithm does not guarantee deterministic stability between executions,
unless the queue structure is controlled (e.g., sorted queue).

For future versions, it may be desirable to:

  - use a priority queue;

  - or preserve the order of node insertion.

example 
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

generates the graph
```text
nodes:
  n0 = prop(count)
  n1 = child(#t0).field(0)
  n2 = child(#t1).field(0)

edges:
  n0 -> n1 []
  n0 -> n2 [f]
```

linear
```text
[n0, n1, n2]

```
another example
```text
nodes:
  n0 = prop(count)
  n1 = child(#t0).field(0)
  n2 = child(#t1).field(0)
  n3 = child(#t2).field(0)

edges:
  n0 -> n1 []
  n0 -> n2 [f]
  n0 -> n3 [f]

```
linear
```text
[n0,n1,n2,n3]
```
