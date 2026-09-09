# Externs

## Overview

Extern values/keywords refer to values and types that **do not exist in the program's source code**, but are provided by the runtime or execution environment for which the code is compiled. Typical examples: `window`, `document`, `console` when the target is a browser.

## Main Idea

When a program needs to access a global object provided by the host (without implementing it), the declaration is marked as external:

```slynx
extern {
    pub object Window {}
    pub object Document {}
    pub object Console {
        pub func log(&self, message: str);
    }
    static window: Window;
    static document: Document;
    static console: Console;
}
```

The only difference from ordinary declarations is that declarations inside an `extern` block **cannot have a body** — they simply reference something that exists at runtime.

## Syntax

```slynx
extern {
    func name(args): ReturnType;
    static NAME: Type;
    pub object Name {
        func method(&self, args): ReturnType;
    }
}
```

Inside the `extern` block:

- functions: signature only, no body;
- objects: fields and methods with signature only;
- statics: typed, no initialization value.

## What can be declared

| Construct | Example |
|-----------|---------|
| Function | `func alert(msg: str): void;` |
| Object | `pub object Window { }` |
| Method on object | `func log(&self, message: str): void;` |
| Static | `static PI: f64;` |
| Static on object | `static document: Document;` |

## Rules

- Declarations in `extern` cannot contain bodies.
- External objects can have methods; methods can take `&self`, `&mut self`, or `self`.
- External methods are invocable via normal object syntax: `console.log("Hello")`.
- External types can reference each other: `Node { func appendChild(self, child: Node): void }`.
- Chained calls are supported: `window.getDocument().getElementById("main")`.

## Examples

```slynx
extern {
    pub object Console {
        pub func log(&self, message: str): void;
    }
    pub object Node {
        pub func appendChild(&self, child: Node): void;
    }
    pub object Document {
        pub func createElement(&self, tag: str): Element;
        pub func getElementById(&self, id: str): Document;
    }
    pub object Element { }
    pub object Window {
        pub func getDocument(&self): Document;
    }

    static window: Window;
    static document: Document;
    static console: Console;
}

func main(): void {
    console.log("Hello, World");
}
```

Chaining:

```slynx
extern {
    pub object Window {
        pub func getDocument(&self): Document;
    }
    pub object Document {
        pub func getElementById(&self, id: str): Element;
    }
    pub object Element { }

    static window: Window;
}

func main(): void {
    let elem = window.getDocument().getElementById("main");
}
```

## Interaction

- The parser uses the `OnlySignatures` flag inside `extern` blocks to skip bodies (`crates/parser/src/declarations.rs`).
- In the HIR, declarations are marked as external (`external: true`).
- In the IR, external functions/statics are marked for external linkage; external statics use `Opcode::GlobalExtern`.

## Summary

- `extern { ... }` declares values/types provided by the runtime/host.
- External declarations have no body.
- External objects can have methods and statics.
- Supports functions, objects, methods, and statics; types can be self-referential and chained.
