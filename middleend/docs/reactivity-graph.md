# Reactivity Graph Foundation

This graph models reactive dependencies as a DAG before they are lowered into IR UI operations.

Planned phase:

1. Frontend/typecheck/component specialization
2. Monomorphization
3. Reactivity graph build + cycle detection + linearization
4. IR bind emission (`@bind`, `@emit`, `@rerender`)

Why keep it as a DAG:

* cycle detection happens before runtime codegen
* linearization stays deterministic for bind emission
* modifiers can travel with each edge and be lowered later as bind transforms
