# Block Comments

## Overview

In addition to line comments (`//`), the lexer supports block comments delimited by `/*` and `*/`, which can span multiple lines.

## Main Idea

Block comments are ignored by the lexer and do not produce tokens in the stream. They can contain any text, including multiple lines.

## Syntax

```slynx
// line comment

/* block comment */

/*
    block comment
    spanning multiple lines
*/
```

## Rules

- Block comments are delimited by `/*` and `*/`.
- They can contain multiple lines.
- The lexer regex does not support nesting of `/* */` inside `/* */`.
- Comments are completely discarded by the lexer (token kind `CommonComent` with `logos::skip`).

## Examples

```slynx
object Person {
    name: str,  /* nome da pessoa */
    /*
        age: int,  // desabilitado por enquanto
    */
    age: int,
}

func main(): int -> 0; // returns 0
```

## Interaction

- The lexer defines two regexes: `//[^\n]*` for line and `/* ... */` for block (`crates/lexer/src/tokens.rs`).
- Both use `logos::skip`, so they do not enter the `TokenStream`.
- They have no effect on the parser, HIR, IR, or codegen.

## Summary

- Block comments: `/* ... */`.
- Line comments: `// ...`.
- Both are ignored by the compiler.
