import fs from "node:fs";
import path from "node:path";

import { describe, expect, test } from "vitest";

import { Monomorphizer, TypeChecker, buildHirFromSource, compileToIr } from "../src/index.js";
import { contextForExample, examplePath, stdPath, tempSource } from "./common.js";

describe("type checking", () => {
  test("function calls work with mixed declaration order", () => {
    const hir = buildHirFromSource("func bar(): void {} func main(): void { bar() }");
    expect(() => TypeChecker.check(hir)).not.toThrow();
  });

  test("rejects function call with wrong argument type", () => {
    const hir = buildHirFromSource("func takes_int(value: int): void {} func main(): void { takes_int(true) }");
    expect(() => TypeChecker.check(hir)).toThrow(/IncompatibleTypes/);
  });

  test("rejects missing return value for non-void functions", () => {
    const hir = buildHirFromSource("func main(): int { let x = 12; }");
    expect(() => TypeChecker.check(hir)).toThrow(/MissingReturnValue/);
  });

  test("rejects while with non-boolean condition", () => {
    const hir = buildHirFromSource("func main(): void { while 10 { 0; } }");
    expect(() => TypeChecker.check(hir)).toThrow(/IncompatibleTypes/);
  });

  test("rejects invalid statement inside while body", () => {
    const hir = buildHirFromSource(
      "func takes_int(value: int): void {} func main(): void { while true { takes_int(false); } }"
    );
    expect(() => TypeChecker.check(hir)).toThrow(/IncompatibleTypes/);
  });

  test("resolves field access through aliases", () => {
    const hir = buildHirFromSource(
      "object Person { age: int } alias PersonAlias = Person; func make_person(): PersonAlias { Person(age: 22) } func main(): int { let person = make_person(); person.age }"
    );
    expect(() => TypeChecker.check(hir)).not.toThrow();
  });

  test("resolves tuple access for tuple variables", () => {
    const hir = buildHirFromSource("func main(): int { let pair = (10, 20); pair.0 }");
    expect(() => TypeChecker.check(hir)).not.toThrow();
  });

  test("resolves named field access after tuple access", () => {
    const hir = buildHirFromSource(
      "object Person { age: int } func main(): int { let pair = (Person(age: 22), \"ok\"); pair.0.age }"
    );
    expect(() => TypeChecker.check(hir)).not.toThrow();
  });

  test("rejects tuple access with invalid index", () => {
    const hir = buildHirFromSource("func main(): int { let pair = (10, 20); pair.2 }");
    expect(() => TypeChecker.check(hir)).toThrow(/InvalidTupleIndex/);
  });

  test("rejects tuple access on non-tuples", () => {
    const hir = buildHirFromSource("func main(): int { let value = 10; value.0 }");
    expect(() => TypeChecker.check(hir)).toThrow(/InvalidTupleAccessTarget/);
  });
});

describe("hir generation and monomorphization", () => {
  test("rejects function call with extra arg during hir build", () => {
    expect(() =>
      buildHirFromSource("func add(a: int, b: int): int { a + b } func main(): void { add(1, 2, 3) }")
    ).toThrow(/InvalidFuncallArgLength/);
  });

  test("rejects function call with missing arg during hir build", () => {
    expect(() =>
      buildHirFromSource("func add(a: int, b: int): int { a + b } func main(): void { add(1) }")
    ).toThrow(/InvalidFuncallArgLength/);
  });

  test("rejects cyclic aliases in monomorphization", () => {
    const hir = buildHirFromSource("alias A = B; alias B = A; func main(): void {}");
    TypeChecker.check(hir);
    expect(() => Monomorphizer.resolve(hir)).toThrow(/RecursiveType/);
  });
});

describe("imports and stylesheets", () => {
  test("import alias and brace imports compile", () => {
    for (const fileName of ["main.slx", "brace_import.slx", "brace_alias_import.slx"]) {
      expect(() => compileToIr(examplePath("imports", fileName), stdPath)).not.toThrow();
    }
  });

  test("wrong module import errors", () => {
    expect(() => compileToIr(examplePath("imports", "test_wrong_module_import.slx"))).toThrow(/ImportError/);
  });

  test("all stylesheet uses compile", () => {
    const dir = examplePath("styles");
    const entries = fs.readdirSync(dir).filter((entry) => entry.endsWith(".slx")).sort();
    for (const entry of entries) {
      expect(() => compileToIr(path.join(dir, entry))).not.toThrow();
    }
  });
});
