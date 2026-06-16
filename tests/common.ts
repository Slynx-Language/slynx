import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { SlynxContext, buildHirFromSource, compileToIr } from "../src/index.js";

const thisFile = fileURLToPath(import.meta.url);
export const repoRoot = path.resolve(path.dirname(thisFile), "..");
export const stdPath = path.join(repoRoot, "lib", "std");

export function examplePath(...segments: string[]): string {
  return path.join(repoRoot, "examples", ...segments);
}

export function compileExample(...segments: string[]) {
  return compileToIr(examplePath(...segments), stdPath);
}

export function contextForExample(...segments: string[]) {
  return SlynxContext.new(examplePath(...segments), stdPath);
}

export function tempSource(source: string, extension = ".syx"): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "slynx-ts-"));
  const filePath = path.join(dir, `input${extension}`);
  fs.writeFileSync(filePath, source, "utf8");
  return filePath;
}

export function tempExampleCopy(exampleName: string): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "slynx-copy-"));
  const filePath = path.join(dir, "input.syx");
  fs.writeFileSync(filePath, fs.readFileSync(examplePath(exampleName), "utf8"), "utf8");
  return filePath;
}

export { buildHirFromSource };
