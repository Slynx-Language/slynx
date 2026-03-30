# Contributing to Slynx

Thanks for taking the time to contribute to Slynx.

This repository is still experimental and the language is still moving, so the best contributions are the ones that improve the codebase without creating unnecessary ambiguity or review overhead.

## Where Contributions Help Most Right Now

The current repository is strongest in these areas:

- `frontend/`: lexer, parser, HIR, and type checker work
- `middleend/`: IR design, lowering, and related tests/specification work
- documentation, specifications, and examples that stay aligned with the current code
- regression tests for parser/checker/middleend behavior

## Before You Start

For larger language, syntax, or IR changes, prefer documenting the direction first through the existing GitHub issue templates or a discussion so the reasoning is visible before implementation.

Relevant project docs:

- [README.md](README.md)
- [GOVERNANCE.md](GOVERNANCE.md)
- [RELEASING.md](RELEASING.md)
- [middleend/README.md](middleend/README.md)
- [docs/issue-reporting.md](docs/issue-reporting.md)

## Local Setup

```bash
git clone https://github.com/Slynx-Language/slynx.git
cd slynx
cargo build
```

## Recommended Workflow

1. Fork the repository.
2. Create a focused branch for one change or one tightly related set of changes.
3. Keep the PR small enough to review comfortably.
4. Update documentation when behavior, workflow, or project expectations change.
5. Use the pull request template and describe both the scope and the validation you ran.

## Validation

For most changes, run the full local validation set before opening the PR:

```bash
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

If your change only affects a specific crate, include the narrower validation command in the PR as well (for example `cargo test -p frontend` or `cargo test -p middleend`).

## Documentation Changes

Documentation contributions are welcome and important.

Good documentation changes should:

- stay aligned with the current codebase
- clearly distinguish implemented behavior from design direction
- avoid promising syntax or tooling that does not exist on `main`
- update related documents when one document changes project-facing behavior or expectations

## Reporting Bugs

Please open a GitHub issue with:

- a clear description of the problem
- reproduction steps
- expected vs actual behavior
- environment details (OS, Rust version, relevant commit/tag)
- a small `.slynx` snippet when applicable

The repository includes dedicated issue templates under [`.github/ISSUE_TEMPLATE/`](.github/ISSUE_TEMPLATE).

If you are unsure how much detail to include, start with [docs/issue-reporting.md](docs/issue-reporting.md).

## Pull Request Expectations

Before opening the PR, make sure that:

- the change is reviewable and scoped
- the validation section is accurate
- new behavior is covered by tests when possible
- related docs were updated when needed
- formatting and clippy were run when applicable

## Releases and Tags

Repository releases are handled by maintainers and should be cut from `main` using `vX.Y.Z` tags.

The repository currently has the public tag `v0.0.1`, and the release process is documented in [RELEASING.md](RELEASING.md).

## Code of Conduct

By participating in the project, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
