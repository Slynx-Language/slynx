import { describe, expect, test } from "vitest";

import { compileToIr } from "../src/index.js";
import { contextForExample, stdPath, tempSource } from "./common.js";

describe("example compilation", () => {
  for (const fileName of [
    "booleans.syx",
    "variables.syx",
    "while.syx",
    "objects.syx",
    "objMethod.syx",
    "objMethods.syx",
    "objMethodStatic.syx",
    "tupleAccess.syx",
    "tupleTwoObjects.syx",
    "tupleNestedObject.syx",
    "numberSystems.syx",
    "commonComments.syx"
  ]) {
    test(`${fileName} compiles`, () => {
      const output = contextForExample(fileName).compile();
      expect(output.outputPath().endsWith(".sir")).toBe(true);
    });
  }

  test("if expression lowering emits control-flow markers", () => {
    const stages = contextForExample("ifExpression.syx").buildStages();
    const ir = stages.irText();

    expect(ir).toContain("Cbr");
    expect(ir).toContain("Br");
    expect(ir).toContain("I32");
  });

  test("inline component source compiles", () => {
    const source = `
component Pedrinho {
    Div {
        Text {
            text: "Pedrinho leitazedo"
        }
    }
}

component Jorgin {
    Text {
        text: "Jorginho neguinho"
    }
    Pedrinho {}
}

func main(): Component {
    Jorgin {}
}
`;
    const filePath = tempSource(source);
    expect(() => compileToIr(filePath)).not.toThrow();
  });

  test("component type mismatch fails", () => {
    expect(() => contextForExample("componentTypeMismatch.syx").compile()).toThrow(/IncompatibleTypes/);
  });
});
