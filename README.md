# Slynx

Experimental UI language frontend rewritten in TypeScript.

## Current Status

The repository is now Node-first and library-first.

What exists on `main`:

- a TypeScript frontend that lexes, parses, builds a lightweight HIR, type-checks, resolves imports, checks alias cycles, and lowers to `SlynxIR`
- library helpers for compiling `.syx` and `.slx` files, writing `.sir` output, and dumping `.hir` / `.ir` stages
- regression coverage for the current example set and the previous Rust integration suite behavior

## Project Layout

- [`src/index.ts`](src/index.ts): compiler pipeline, public API, IR formatting
- [`tests/`](tests): Vitest coverage for compilation, parser/HIR behavior, type checking, imports, and stylesheets
- [`examples/`](examples): real language samples used by tests
- [`lib/std/`](lib/std): standard library fixtures loaded by `import std`
- [`docs/`](docs): language and project documentation

## Getting Started

### Prerequisites

- Node.js 24+
- npm 11+

### Install and Validate

```bash
git clone https://github.com/Slynx-Language/slynx.git
cd slynx
npm install
npm run check
npm test
npm run build
```

## Library Usage

Compile a source file directly to IR:

```ts
import { compileToIr } from "slynx";

const ir = compileToIr("examples/component.syx");
console.log(ir.formatSir());
```

Write the sibling `.sir` file:

```ts
import { compileCode } from "slynx";

compileCode("examples/component.syx");
```

Inspect intermediate dumps:

```ts
import { SlynxContext } from "slynx";

const stages = SlynxContext.new("examples/booleans.syx").buildStages();

console.log(stages.hirText());
stages.writeHir();
stages.writeIr();
stages.intoOutput().write();
```

## Example Sources

- [`examples/component.syx`](examples/component.syx)
- [`examples/objects.syx`](examples/objects.syx)
- [`examples/while.syx`](examples/while.syx)
- [`examples/functionCall.syx`](examples/functionCall.syx)

## Documentation

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [RELEASING.md](RELEASING.md)
- [docs/language-surface.md](docs/language-surface.md)
- [docs/grammar.md](docs/grammar.md)
- [docs/imports.md](docs/imports.md)

## License

MIT. See [LICENSE](LICENSE).
