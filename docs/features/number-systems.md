# Number Systems

## Overview

Integer literals in Slynx can be written in four number bases: decimal, hexadecimal (`0x`), binary (`0b`), and octal (`0o`). Underscore separators (`_`) are accepted to improve the readability of large literals.

## Main Idea

The lexer recognizes the base prefix and converts the literal directly to the integer value. Underscores are ignored during parsing.

## Syntax

| Base | Prefix | Examples |
|------|---------|----------|
| Decimal | none | `42`, `0`, `1_000_000` |
| Hexadecimal | `0x` | `0xFF`, `0x2A`, `0x00ff00` |
| Binary | `0b` | `0b1010`, `0b0110`, `0b1111_0000` |
| Octal | `0o` | `0o77`, `0o071`, `0o172` |

## Rules

- All integer literals are 32-bit signed (`i32`).
- Underscores may appear between digits, but not in positions that cause ambiguity:
  - they cannot be immediately before or after the decimal point of a float (`1._0` and `1_.0` are invalid);
  - a literal cannot end with an underscore (`1_` is invalid).
- Sequences like `4..0` are rejected as malformed numbers by the lexer.
- Literals outside the range of `i32` that cannot be converted are rejected.

## Examples

```slynx
func main(): int {
    let hex = 0x2A;       // 42
    let bin = 0b1010;     // 10
    let oct = 0o77;       // 63
    let dec = 1_000_000;  // 1.000.000
    let sep = 2_000_0_0_0; // 2.000.000
    dec
}
```

Number in expressions:

```slynx
func color(): int -> 0x00ff00;
```

## Interaction

- The lexer uses separate regexes for each base (`crates/lexer/src/tokens.rs`).
- Values are stored as `i32`.
- Numbers can appear in arithmetic expressions, style arguments (`Fg(0xff0000)`), bitwise operations, etc.

## Summary

- 4 bases supported: decimal, hex, binary, octal.
- Underscores allowed for readability.
- All integers are `i32` (32-bit signed).
- Out-of-range values or misplaced underscores are rejected as `MalformedNumber`.
