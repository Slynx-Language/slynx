# Slynx

> A data-oriented UI language for building cross-platform interfaces

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

Slynx is an experimental programming language designed to create user interfaces that compile to multiple targets from a single codebase. It emphasizes readability, performance through data-oriented design, and platform independence.

## Table of Contents

- [Why Slynx?](#why-slynx)
- [Features](#features)
- [Quick Start](#quick-start)
- [Example](#example)
- [Building from Source](#building-from-source)
- [Project Status](#project-status)
- [Contributing](#contributing)
- [Inspirations](#inspirations)
- [License](#license)

## Why Slynx?

### The Problem

Modern UI frameworks face two major challenges:

1. **Poor Readability**: Complex frameworks like JSX make it difficult to understand what's displayed on screen
2. **Platform Lock-in**: Code written for one platform rarely works on another without significant changes

### The Solution

Slynx addresses these issues through:

- **Clear Syntax**: Designed to be immediately understandable. If you look at Slynx code and think "what am I looking at?", we've failed.
- **Data-Oriented Design**: Uses Structure of Arrays (SoA) by default for better performance, with Array of Structures (AoS) available when explicitly needed
- **Cross-Platform Compilation**: Write once, compile to multiple targets (Web, native, mobile) through exposed IR and pluggable backends
- **Frontend as Library**: The compiler frontend is designed as a reusable library that generates IR for custom backends

## Features

- Component-based architecture
- Strong type inference (Hindley-Milner)
- Data-oriented by default
- Reactive property system
- Style inheritance
- Pattern matching (planned)
- Async/await support (planned)

## Quick Start

### Prerequisites

- Rust 1.70 or higher
- Cargo

### Installation

```bash
git clone https://github.com/cykna/slynx.git
cd slynx
cargo build --release
```

### Your First Program

Create a file `hello.slx`:

```slynx
component HelloWorld {
  Div {
    P {
      text: "Hello, Slynx!",
    }
  }
}

func main():Component -> HelloWorld;
```

Compile and run:

```bash
./target/release/Slynx --target hello.slx
```

Or using `just`:

```bash
just run hello.slx
```

## Example

Here's a simple counter component demonstrating Slynx's syntax:

```slynx
style AppText(size: int, color:int) {
  styleprop font_family = "Sans Serif";
  styleprop font_size = size;
  styleprop text_color = color;
}

style Rounded(pixels:int){
  styleprop border_radius = pixels;
}

style RoundedCounter(color:int, round:int) inherits Rounded(round) {
  styleprop color = color;
}

static style GeneralText = AppText(16,0xffffff);

component Counter {
  prop value = 0;
  pub(parent) style: Style?;
  
  Div {
    style: style matches Some(s) ? s : RoundedCounter(0xff0000, 24),
    P {
      text: value,
      style: GeneralText
    }
    Button {
      text: "Increase to be $(value+1)",
      style: GeneralText,
      on_click: event -> value += 1;
    }
  }
}

component AlotOfCounters<N:const int > 0>{
  for i in 0..N do if i % 2 {
    Counter{}
  };
}

func main():Component -> AlotOfCounters<20>;
```

## Building from Source

```bash
# Clone the repository
git clone https://github.com/cykna/slynx.git
cd slynx

# Build in release mode
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Project Status

**Current State**: Experimental

The project is in active development with focus on:
- Frontend implementation (lexer, parser, type system)
- JavaScript code generation
- Proving the multi-target compilation concept

### Roadmap

Legend: `[x]` Done | `[~]` In Progress | `[ ]` Planned

#### Language Design
- `[~]` Core vision and goals
- `[~]` UI-oriented component model
- `[~]` Function definitions (focusing on Components)
- `[~]` Numeric and string types
- `[ ]` Boolean type
- `[~]` Object model
- `[ ]` Structs
- `[ ]` Enums
- `[ ]` Serialization/Deserialization
- `[ ]` Control flow model (if/match/loops)
- `[ ]` Error model

#### Frontend Implementation
- `[~]` Lexer (identifiers, keywords, numbers, strings)
- `[~]` Parser (expressions, functions, components, objects)
- `[~]` Type system with Hindley-Milner inference
- `[ ]` Generic parameters
- `[ ]` Pattern matching
- `[ ]` Error recovery

#### Backend & IR
- `[ ]` SSA-based IR design
- `[ ]` Data-oriented layout (SoA by default)
- `[~]` JavaScript backend
- `[ ]` Runtime support (reactivity, events)
- `[ ]` DOM/UI binding

#### Tooling
- `[~]` CONTRIBUTING.md
- `[ ]` Language reference documentation
- `[ ]` IR specification
- `[ ]` Example applications

For detailed roadmap, see [ROADMAP.md](ROADMAP.md) (if exists).

## Contributing

We welcome contributions! Slynx requires significant work to reach maturity, and we can't do it alone.

**How to contribute:**
- Read our [Contributing Guide](CONTRIBUTING.md)
- Check out [good first issues](https://github.com/cykna/slynx/labels/good%20first%20issue)
- Join discussions in [GitHub Issues](https://github.com/cykna/slynx/issues)

Areas where we need help:
- Frontend development (parser, type system)
- Backend implementations
- Documentation
- Testing
- Example applications

## Inspirations

### UI Design
- **QML**: Component-based UI declaration
- **Slint**: Modern UI language design

### Language Features
- **Rust**: Type system, ownership concepts, error handling
- **Swift**: Syntax clarity, property observers
- **JavaScript**: Async/await, flexibility

## Architecture

Slynx separates the frontend (parser, type checker) from backends (code generators):

```
Slynx Source → Frontend → IR → Backend → Target Code
                                 ↓
                          (Web, Native, Mobile)
```

This architecture allows:
- Consistent error checking across all platforms
- Community-driven backend development
- Easy addition of new compilation targets

## Community

- **Issues**: Report bugs and request features on [GitHub Issues](https://github.com/cykna/slynx/issues)
- **Discussions**: Join conversations in [GitHub Discussions](https://github.com/cykna/slynx/discussions)
- **Contributing**: See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.


## Acknowledgments

Thanks to all contributors who help make Slynx better!

---

**Note**: Slynx is experimental software under active development. APIs and language features may change significantly.
