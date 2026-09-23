# Deferred: Driver ergonomics and pipeline staging (Phase 6 architectural work)

Status: DEFERRED - documented, not implemented. See docs/refactors/3.4-method-mangling.md
for the deferral convention this file follows.

## What is deferred

Phase 6 mechanical subsets are DONE (Phase 6.1 subset: driver phase-boundary leakiness):
- `build_stages` now delegates to `load_modules` instead of re-implementing the SourceLoader
  loop (dead code removed, src/compilation_context/mod.rs:382-405 -> load_modules).
- Dead `build_tokens`/`build_parser`/`build_parser` helpers and unused imports removed.

Deferred architectural items in the same area:

### 6.1-architectural: no MonomorphizedHir boundary type, no pipeline module
Location: src/compilation_context/mod.rs:382-405 (build_stages)
Problem: Monomorphization runs inside build_hir (:342-360) rather than as a separate
stage, and the monomorphizer mutates the HIR in place. build_stages returns
CompilationStages (a bundle) so callers can inspect intermediate dumps, but there is no
explicit `MonomorphizedHir` boundary type and no named pipeline module documenting each
phase transition.
Contained/immediately-doable variants: Adding a `MonomorphizedHir` newtype alias and
naming pipeline phases in a `pipeline` module. Doing this well overlaps the deferred
HIR-pipeline redesign (phases 1.2 and 3.1 in AcrossCodebase.md), so it is deferred with
them to avoid two competing rewrites of the same driver.

### 6.2: A two-phase stylesheet pass does the same work twice
Location: crates/codegen/src/lib.rs:232-257 (stylesheet_pre_pass) vs
crates/codegen/src/helper/styles.rs:75-79 (helper/styles.rs)
Problem: stylesheet_pre_pass computes inheritance + property codes in phase 0, then
lower_stylesheet at styles.rs:75-79 recomputes and overwrites the same values. Same work
shaped twice.
Suggested fix (deferred): make the pre_pass the single place that resolves inheritance so
lower_stylesheet consumes the result instead of recomputing it.
Note: touching inheritance resolution is semantically sensitive (property inheritance
order, cascading rules); deferred until stylesheet lowering is covered by dedicated
golden tests (Phase 8.3 work).

### 6.4: impure_instructions is a global stream without invalidation guards
Location: crates/ir/src/ir.rs:44, labels slice instruction_start..start+count into the
global impure_instructions.
Problem: Coordinates are global and silently break if functions aren't lowered strictly
sequentially; no check that a label's range still matches after other functions are
appended. Doc says "flat instruction array" (label.rs:7-9) while views carve
impure_instructions (views/function.rs:7-9) - docs and implementation disagree.
Suggested fix (deferred): refactor to per-function impure ranges held next to each
function's label, validated on append. Requires IR model change; deferred with the IR
phase OI-mapping consolidation.

## Unsafe-guard constraints honored while deferring
- No semantic changes: 448-line driver and stylesheet/IR behavior identical.
- Tests green: 26/26 test binaries pass; clippy `-D warnings` clean.
