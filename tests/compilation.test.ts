import fs from "node:fs";
import path from "node:path";

import { describe, expect, test } from "vitest";

import { SlynxContext, compileCode } from "../src/index.js";
import { buildHirFromSource, stdPath, tempExampleCopy } from "./common.js";

describe("compilation output", () => {
  test("compile returns output before writing", () => {
    const sourcePath = tempExampleCopy("booleans.syx");
    const outputPath = sourcePath.replace(/\.syx$/, ".sir");

    const context = SlynxContext.new(sourcePath);
    const output = context.compile();

    expect(output.outputPath()).toBe(outputPath);
    expect(fs.existsSync(outputPath)).toBe(false);

    output.write();
    expect(fs.existsSync(outputPath)).toBe(true);
  });

  test("compileCode writes .sir output", () => {
    const sourcePath = tempExampleCopy("booleans.syx");
    const outputPath = sourcePath.replace(/\.syx$/, ".sir");

    compileCode(sourcePath, stdPath);

    expect(fs.existsSync(outputPath)).toBe(true);
    expect(fs.readFileSync(outputPath, "utf8")).not.toHaveLength(0);
  });

  test("buildStages exposes dumps without writing files", () => {
    const sourcePath = tempExampleCopy("booleans.syx");
    const hirPath = sourcePath.replace(/\.syx$/, ".hir");
    const irPath = sourcePath.replace(/\.syx$/, ".ir");

    const stages = SlynxContext.new(sourcePath).buildStages();

    expect(stages.dumpPath("hir")).toBe(hirPath);
    expect(stages.dumpPath("ir")).toBe(irPath);
    expect(stages.hirText()).toContain("HIR Files");
    expect(stages.irText()).not.toHaveLength(0);
    expect(fs.existsSync(hirPath)).toBe(false);
    expect(fs.existsSync(irPath)).toBe(false);
  });

  test("buildStages can write hir, ir and sir", () => {
    const sourcePath = tempExampleCopy("booleans.syx");
    const hirPath = sourcePath.replace(/\.syx$/, ".hir");
    const irPath = sourcePath.replace(/\.syx$/, ".ir");
    const sirPath = sourcePath.replace(/\.syx$/, ".sir");

    const stages = SlynxContext.new(sourcePath).buildStages();
    stages.writeHir();
    stages.writeIr();
    stages.intoOutput().write();

    for (const filePath of [hirPath, irPath, sirPath]) {
      expect(fs.existsSync(filePath)).toBe(true);
      expect(fs.readFileSync(filePath, "utf8")).not.toHaveLength(0);
    }
  });
});

describe("hir surface", () => {
  test("preserves non-expression tail statements in function bodies", () => {
    const hir = buildHirFromSource("func main(): void { let x = 12; }");
    const declarations = hir.files[0].read().declarations();
    const mainFn = declarations[0];

    expect(mainFn.kind).toBe("function");
    if (mainFn.kind !== "function" || mainFn.body.kind !== "block") {
      throw new Error("unexpected declaration shape");
    }

    expect(mainFn.body.statements).toHaveLength(1);
    expect(mainFn.body.statements[0]?.kind).toBe("let");
    expect(mainFn.body.tail).toBeUndefined();
  });
});
