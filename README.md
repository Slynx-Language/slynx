# Slynx

> Experimental UI language workspace focused on the frontend and middleend of the language.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)

Slynx is an experimental programming language project for user interfaces. The long-term direction is to expose a reusable IR that downstream compilers can consume, but the current repository is primarily a library-first workspace for the language frontend and middleend.

## Current Status

Slynx is still experimental and under active design.

What is true on the current `main` branch:

- the workspace is library-first; there is no official CLI binary target in `main` right now
- the root crate can lex, parse, build HIR, run type checking, and lower source files into `IntermediateRepr`
- the root library can write the current IR output to a sibling `.sir` file
- sample `.slynx` sources live under [`slynx/`](slynx)
- the IR design is still evolving, and the design reference in [`middleend/README.md`](middleend/README.md) is broader than what `main` emits today

## Workspace Layout

The repository is currently split into these crates:

- [`common/`](common): shared AST types and common language data structures
- [`frontend/`](frontend): lexer, parser, HIR generation, and type checking
- [`middleend/`](middleend): `IntermediateRepr` and IR/lowering work
- [`src/`](src): root library glue (`SlynxContext`, compile helpers, error presentation)

## What Exists Today

The current codebase already includes:

- lexical analysis and parsing for core language constructs such as functions, objects, components, and basic control flow
- HIR generation and name resolution
- type checking and type inference for the current supported surface
- lowering to the current `IntermediateRepr`
- CI, release, governance, and contribution documentation

The current repository does **not** ship an official backend crate or an official CLI binary on `main`.

## Getting Started

### Prerequisites

- Rust stable
- Cargo

### Build and Validate the Workspace

```bash
git clone https://github.com/Slynx-Language/slynx.git
cd slynx
cargo build
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

### Using the Library Today

The root crate currently exposes helper functions for lowering a `.slynx` file into IR:

```rust
use std::path::PathBuf;

fn main() -> color_eyre::eyre::Result<()> {
    let ir = slynx::compile_to_ir(PathBuf::from("slynx/component.slynx"))?;
    println!("{ir:#?}");
    Ok(())
}
```

If you want to write the current IR output to disk, use `slynx::compile_code(...)` or `SlynxContext::start_compilation(...)`. The output is written next to the input file with the `.sir` extension.

## Example Source

A small sample that matches the current repository syntax lives in [`slynx/component.slynx`](slynx/component.slynx):

```slynx
component HelloBox {
    pub prop seupai: float;
    pub prop maria = 0;

    Div {
        Text {
            text: maria
        }
    }
}

func main(): Component {
    HelloBox {
        seupai: 5.0
        maria: 12
    }
}
```

## Documentation Map

Core project documents:

- [CONTRIBUTING.md](CONTRIBUTING.md): contribution workflow and validation expectations
- [GOVERNANCE.md](GOVERNANCE.md): project roles and decision structure
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md): community behavior expectations
- [RELEASING.md](RELEASING.md): how repository tags and GitHub Releases are cut
- [CHANGELOG.md](CHANGELOG.md): repository-level changelog
- [middleend/README.md](middleend/README.md): IR design/specification reference
- [docs/landing-content-inventory.md](docs/landing-content-inventory.md): grounded inventory of what can already be published on the landing page

Operational templates:

- [.github/pull_request_template.md](.github/pull_request_template.md)
- [.github/ISSUE_TEMPLATE/bug_report.md](.github/ISSUE_TEMPLATE/bug_report.md)
- [.github/ISSUE_TEMPLATE/feature_request.md](.github/ISSUE_TEMPLATE/feature_request.md)
- [.github/ISSUE_TEMPLATE/documentation.md](.github/ISSUE_TEMPLATE/documentation.md)
- [.github/ISSUE_TEMPLATE/discussion.md](.github/ISSUE_TEMPLATE/discussion.md)

## Releases and Versioning

The repository currently has a public tag, [`v0.0.1`](https://github.com/Slynx-Language/slynx/tags), created on 2026-03-21.

At the time of writing, there is still no published GitHub Release page for that tag:

- [GitHub Releases](https://github.com/Slynx-Language/slynx/releases)
- [Git tags](https://github.com/Slynx-Language/slynx/tags)

For the release process itself, see [RELEASING.md](RELEASING.md).

## Contributing

Contributions are welcome, especially in these areas:

- frontend/parser/type-checker work
- IR and middleend design
- tests and regression coverage
- documentation and specifications

Start with [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR.

## Community

- [GitHub Issues](https://github.com/Slynx-Language/slynx/issues)
- [GitHub Discussions](https://github.com/Slynx-Language/slynx/discussions)

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
