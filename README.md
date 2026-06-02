# Slynx

> Experimental UI language workspace focused on the frontend and middleend of the language.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)

Slynx is an experimental programming language project for user interfaces. The long-term direction is to expose a reusable IR that downstream compilers can consume, but the current repository is primarily a library-first workspace for the language frontend and middleend.

## Current Status

Slynx is still experimental and under active design.

What is true on the current `main` branch:

- the workspace is library-first; there is no official CLI binary target in `main` right now
- the root crate can lex, parse, build HIR, run type checking, resolve the current alias surface, and lower source files into `SlynxIR`
- the root library can write the default `.sir` output and can expose `.hir` / `.ir` dumps through `SlynxContext::build_stages()`
- sample `.syx` sources live under [`examples/`](examples)
- the IR design is still evolving, and the design reference in [`crates/slynx_ir/README.md`](crates/slynx_ir/README.md) is broader than what `main` emits today

## Workspace Layout

The repository is a Cargo workspace. The root crate (`src/`) is the library entry point; the individual pipeline stages live under `crates/`:

- [`crates/common/`](crates/common): shared AST types and common language data structures
- [`crates/lexer/`](crates/lexer): lexical analysis (`slynx-lexer`)
- [`crates/parser/`](crates/parser): parser (`slynx-parser`)
- [`crates/hir/`](crates/hir): HIR generation and name resolution (`slynx-hir`)
- [`crates/checker/`](crates/checker): type checking and type inference (`slynx-typechecker`)
- [`crates/monomorphizer/`](crates/monomorphizer): monomorphization (`slynx-monomorphizer`)
- [`crates/slynx_ir/`](crates/slynx_ir): `SlynxIR` definition and lowering (`slynx-ir`)
- [`src/`](src): root library glue (`SlynxContext`, compile helpers, error presentation)

## What Exists Today

The current codebase already includes:

- lexical analysis and parsing for core language constructs such as functions, objects, components, aliases, tuple literals/types, and control flow such as `if` and `while`
- HIR generation and name resolution
- type checking, type inference, and monomorphization for the current supported alias surface
- library entry points for compiling to IR or inspecting HIR/IR dumps before writing output
- lowering to the current `SlynxIR`
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

The root crate exposes helper functions for lowering a `.syx` file into IR:

```rust
use std::path::PathBuf;

fn main() -> color_eyre::eyre::Result<()> {
    let ir = slynx::compile_to_ir(PathBuf::from("examples/component.syx"))?;
    println!("{ir:#?}");
    Ok(())
}
```

To write the default `.sir` output file alongside the source:

```rust
use std::path::PathBuf;

fn main() -> color_eyre::eyre::Result<()> {
    slynx::compile_code(PathBuf::from("examples/component.syx"))?;
    Ok(())
}
```

If you want to inspect intermediate dumps before writing output, the root context also exposes stage building:

```rust
use std::{path::PathBuf, sync::Arc};

fn main() -> color_eyre::eyre::Result<()> {
    let context = slynx::SlynxContext::new(Arc::new(PathBuf::from("examples/booleans.syx")))?;
    let stages = context.build_stages()?;

    println!("{}", stages.hir_text());
    stages.write_hir()?;
    stages.write_ir()?;

    let output = stages.into_output();
    output.write()?;
    Ok(())
}
```

Today:

- `compile_code(...)` writes the default sibling `.sir` file
- `compile_to_ir(...)` returns the compiled `SlynxIR` directly
- `build_stages()` lets callers inspect or persist `.hir` and `.ir` dumps through the library API
- there is still no polished CLI workflow for dump generation on `main`

## Example Sources

Real samples that match the current repository syntax live under [`examples/`](examples), for example:

- [`examples/component.syx`](examples/component.syx): basic component construction
- [`examples/objects.syx`](examples/objects.syx): object construction and field mutation
- [`examples/while.syx`](examples/while.syx): `while` loops
- [`examples/functionCall.syx`](examples/functionCall.syx): typed function calls

One small component example:

```slynx
component Header {
    Div {
        Icon {
            src: "https://github.icon.this.url.does_not_exist.com"
        }
    }
}

component Footer {
    Div {
        Text {text: "Footer of page"}
    }
}

component Main {
    Div {
        Text {text: "Main Part"}
    }
}

component Website {
    Div {
        Header {}
        Main {}
        Footer {}
    }
}

func main(): Component {
    Website {}
}
```

## Documentation Map

Core project documents:

- [CONTRIBUTING.md](CONTRIBUTING.md): contribution workflow and validation expectations
- [GOVERNANCE.md](GOVERNANCE.md): project roles and decision structure
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md): community behavior expectations
- [RELEASING.md](RELEASING.md): how repository tags and GitHub Releases are cut
- [CHANGELOG.md](CHANGELOG.md): repository-level changelog
- [docs/language-surface.md](docs/language-surface.md): grounded overview of the current language syntax and constructs
- [docs/first-slynx-file.md](docs/first-slynx-file.md): short tutorial-style example for new contributors
- [crates/slynx_ir/README.md](crates/slynx_ir/README.md): IR design/specification reference
- [docs/issue-reporting.md](docs/issue-reporting.md): guide for opening clear, actionable issues
- [docs/landing-content-inventory.md](docs/landing-content-inventory.md): grounded inventory of what can already be published on the landing page

Operational templates:

- [.github/pull_request_template.md](.github/pull_request_template.md)
- [.github/ISSUE_TEMPLATE/bug_report.md](.github/ISSUE_TEMPLATE/bug_report.md)
- [.github/ISSUE_TEMPLATE/feature_request.md](.github/ISSUE_TEMPLATE/feature_request.md)
- [.github/ISSUE_TEMPLATE/documentation.md](.github/ISSUE_TEMPLATE/documentation.md)
- [.github/ISSUE_TEMPLATE/discussion.md](.github/ISSUE_TEMPLATE/discussion.md)

## Releases and Versioning

The latest release is [`v0.0.1`](https://github.com/Slynx-Language/slynx/releases/tag/v0.0.1), published on 2026-05-04.

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
- [Discord Server](https://discord.gg/B4pHKHqNxG)

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
