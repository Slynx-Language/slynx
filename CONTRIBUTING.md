# Contributing to Slynx

Slynx is experimental, but the repository is now TypeScript-first. Contributions should keep the current parser/type-checker behavior precise and testable.

## Local Setup

```bash
git clone https://github.com/Slynx-Language/slynx.git
cd slynx
npm install
```

## Validation

Run the full local validation set before opening a PR:

```bash
npm run check
npm test
npm run build
```

## Contribution Priorities

- parser and type-checker improvements in [`src/index.ts`](src/index.ts)
- regression tests in [`tests/`](tests)
- docs and examples that stay aligned with shipped behavior
- import, stylesheet, and IR surface refinements that do not regress existing fixtures

## Pull Request Expectations

- keep the scope reviewable
- update tests for behavior changes
- update docs when developer workflow or language behavior changes
- report only validation you actually ran

## Code of Conduct

By participating in the project, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
