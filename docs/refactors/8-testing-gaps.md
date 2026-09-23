# 8. Testing Gaps — deferral record (with proof)

Status: DOCUMENTED-DEFERRED. No code changes made (tree left byte-identical to green HEAD).
Location (doc source): docs/refactors/AcrossCodebase.md §8.1-8.5.
Companion pattern: docs/refactors/3.4-method-mangling.md, 6-driver-pipeline.md, 4.1-visit-walk.md.

## Why the whole phase is deferred (not mechanical-executed now)
- 8.1 (five of eight crates have zero unit tests) + 8.2 (only ~3 of 30+ HIRErrorKind
  variants directly tested): fixes require AUTHORING ~30 new green-by-construction unit
  tests, each needing a known-good reachable path through lowerers/monomorphizer. If any
  chosen path hits a real bug (like 8.4's, proven below), the added test is RED and the
  bug gets silently "fixed" by weakening the test — the exact anti-pattern 8.3/8.4 warns
  about. Not mechanical; deferred pending a behavior audit first.
- 8.3 (IR/SIR output almost never asserted): fixing = introducing a golden/snapshot SIR
  dump harness = new test infrastructure, expressly out of scope for a mechanical refactor.
- 8.4 (currently-broken paths uncovered): the correct action for the known-wrong
  monomorphizer test is a REAL fix to Monomorphizer arity validation, which is a semantic/
  behavioral change — deferred together with the ownership/monomorphizer phase work.
- 8.5 (data-driven xpass/xfail breadth hides depth): by design of the existing suite; a
  covering change is the same snapshot-infra work as 8.3.

## PROOF (obtained this session, rustc-authoritative)
tests/monomorphizer.rs:163 — `rejects_wrong_generic_arity` calls:
    Monomorphizer::resolve(&mut hir).expect("expected generic to be properly inferred");
  for source `second<int>(1, 2)` against `func second<T, U>(first: T, second: U): U`.
Experiment: flipping the expectation to `assert!(result.is_err(), ...)` FAILS immediately
(cargo test, monomorphizer bin, exit=101). Conclusion: Monomorphizer::resolve currently
returns Ok for a wrong generic-arity call — arity is NOT validated. The test still passes
only because it asserts the success string. The suite's 26-green therefore does NOT cover
generic-arity rejection.

## Single unblock
Add arity-count validation inside crates/monomorphizer (compare applied type-arg count vs
the generic parameter count before instantiating). That is the one-place semantic fix that
turns this test into an honest rejection test конца. Defer to the monomorphizer/ownership
architectural phase; record as blocker for 8.2+8.4 tick marks.
