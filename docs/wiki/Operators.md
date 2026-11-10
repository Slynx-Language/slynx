# Operators

Operators are symbols that combine values into expressions. Slynx uses conventional operator syntax for arithmetic, comparison, logical, and bitwise operations. The binary operators described on this page each take a left-hand operand and a right-hand operand; compound assignment operators additionally write the result back to the left-hand operand.

This page documents the binary and compound assignment operators currently supported by Slynx. Plain assignment (`=`) is not a binary operator; the equality operator is `==`. Unary operators are outside the scope of this page. Compound assignment operators such as `+=` are equivalent to assigning the corresponding binary expression back to the left-hand side (for example, `lhs += rhs` is `lhs = lhs + rhs`).

## Arithmetic operators

Arithmetic operators perform numeric calculations. Slynx currently provides addition, subtraction, multiplication, division, and remainder.

### Addition (`+`)

Syntax: `lhs + rhs`

The `+` operator adds its two operands.

```slx
func add(a: sint32, b: sint32): sint32 -> a + b;
```

### Subtraction (`-`)

Syntax: `lhs - rhs`

The `-` operator subtracts the right-hand operand from the left-hand operand.

```slx
func subtract(a: sint32, b: sint32): sint32 -> a - b;
```

### Multiplication (`*`)

Syntax: `lhs * rhs`

The `*` operator multiplies its two operands.

```slx
func multiply(a: sint32, b: sint32): sint32 -> a * b;
```

### Division (`/`)

Syntax: `lhs / rhs`

The `/` operator divides the left-hand operand by the right-hand operand.

```slx
func divide(a: sint32, b: sint32): sint32 -> a / b;
```

The behavior of division when the right-hand operand is zero is not yet documented.

### Remainder (`%`)

Syntax: `lhs % rhs`

The `%` operator produces the remainder of dividing the left-hand operand by the right-hand operand. It is a remainder operator, not necessarily a mathematical modulo operator. The exact result for negative operands is not yet documented.

```slx
func remainder(a: sint32, b: sint32): sint32 -> a % b;
```

## Comparison operators

Comparison operators compare two values and produce a `bool`. The equality operator (`==`) tests whether two values are equal. The relational operators (`>`, `>=`, `<`, and `<=`) test the ordering of two values. All five comparison operators produce a `bool` result.

The single `=` character is assignment syntax, not equality. The equality operator is `==`.

### Equality (`==`)

Syntax: `lhs == rhs`

The `==` operator evaluates to `true` when both operands are equal, and to `false` otherwise.

```slx
func same(a: sint32, b: sint32): bool -> a == b;
```

### Greater than (`>`)

Syntax: `lhs > rhs`

The `>` operator evaluates to `true` when the left-hand operand is greater than the right-hand operand.

```slx
func greater(a: sint32, b: sint32): bool -> a > b;
```

### Greater than or equal (`>=`)

Syntax: `lhs >= rhs`

The `>=` operator evaluates to `true` when the left-hand operand is greater than or equal to the right-hand operand.

```slx
func can_vote(age: sint32): bool -> age >= 18;
```

### Less than (`<`)

Syntax: `lhs < rhs`

The `<` operator evaluates to `true` when the left-hand operand is less than the right-hand operand.

```slx
func less(a: sint32, b: sint32): bool -> a < b;
```

### Less than or equal (`<=`)

Syntax: `lhs <= rhs`

The `<=` operator evaluates to `true` when the left-hand operand is less than or equal to the right-hand operand.

```slx
func at_most(a: sint32, b: sint32): bool -> a <= b;
```

## Logical operators

Logical operators operate on `bool` values and produce a `bool`. They are distinct from the bitwise operators `&` and `|`: `&&` and `||` combine boolean values, while `&` and `|` operate on the bits of their operands.

### Logical AND (`&&`)

Syntax: `lhs && rhs`

The `&&` operator produces `true` when both operands are `true`, and `false` otherwise.

```slx
func both(a: bool, b: bool): bool -> a && b;
```

### Logical OR (`||`)

Syntax: `lhs || rhs`

The `||` operator produces `true` when at least one operand is `true`, and `false` otherwise.

```slx
func either(a: bool, b: bool): bool -> a || b;
```

Whether `&&` and `||` short-circuit is not yet documented.

## Bitwise operators

Bitwise operators operate on the bit patterns of their operands. They are not the same as the logical operators: `&` is bitwise AND, while `&&` is logical AND; `|` is bitwise OR, while `||` is logical OR.

The exact set of types accepted by the bitwise operators is not yet documented. The examples below use `uint8`.

### Bitwise AND (`&`)

Syntax: `lhs & rhs`

The `&` operator combines the bits of its operands and keeps a bit set only when the corresponding bit is set in both operands.

```slx
func mask(value: uint8, flag: uint8): uint8 -> value & flag;
```

### Bitwise OR (`|`)

Syntax: `lhs | rhs`

The `|` operator combines the bits of its operands and sets a bit when the corresponding bit is set in either operand.

```slx
func combine(a: uint8, b: uint8): uint8 -> a | b;
```

### Bitwise XOR (`^`)

Syntax: `lhs ^ rhs`

The `^` operator combines the bits of its operands and sets a bit when the corresponding bits differ.

```slx
func toggle(a: uint8, b: uint8): uint8 -> a ^ b;
```

## Bitwise shift operators

Shift operators move the bits of the left-hand operand by the number of positions given by the right-hand operand.

The exact accepted operand types, as well as the behavior for shift amounts that are negative or larger than the operand width, are not yet documented.

### Left shift (`<<`)

Syntax: `lhs << rhs`

The `<<` operator shifts the bits of the left-hand operand to the left.

```slx
func shift_left(value: uint8, amount: uint8): uint8 -> value << amount;
```

### Right shift (`>>`)

Syntax: `lhs >> rhs`

The `>>` operator shifts the bits of the left-hand operand to the right.

```slx
func shift_right(value: uint8, amount: uint8): uint8 -> value >> amount;
```

## Compound assignment operators

Compound assignment operators combine a binary operation with an assignment. The syntax `lhs op= rhs` is equivalent to assigning the corresponding binary expression back to the left-hand side:

```text
lhs += rhs   is equivalent to   lhs = lhs + rhs
```

The left-hand side must be an assignable expression, such as a mutable variable, a field access, or a dereference. Compound assignment is a statement form; it is not an expression and therefore has no place in the expression precedence table. There are no compound forms for the comparison operators.

```slx
func accumulate(): void {
    let mut total: sint32 = 0;
    total += 10;
    total *= 2;
    total %= 7;
}
```

The supported compound operators are:

| Syntax | Equivalent to | Category |
|--------|---------------|----------|
| `+=` | `lhs = lhs + rhs` | Arithmetic |
| `-=` | `lhs = lhs - rhs` | Arithmetic |
| `*=` | `lhs = lhs * rhs` | Arithmetic |
| `/=` | `lhs = lhs / rhs` | Arithmetic |
| `%=` | `lhs = lhs % rhs` | Arithmetic |
| `&&=` | `lhs = lhs && rhs` | Logical |
| `\|\|=` | `lhs = lhs \|\| rhs` | Logical |
| `&=` | `lhs = lhs & rhs` | Bitwise |
| `\|=` | `lhs = lhs \| rhs` | Bitwise |
| `^=` | `lhs = lhs ^ rhs` | Bitwise |
| `<<=` | `lhs = lhs << rhs` | Bitwise shift |
| `>>=` | `lhs = lhs >> rhs` | Bitwise shift |

## Operator precedence and associativity

The current parser groups binary operators into the following precedence levels. Operators in a higher row bind more tightly.

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 1 (highest) | `*` `/` `%` | Left |
| 2 | `+` `-` | Left |
| 3 | `<<` `>>` `&` `^` `\|` | Right |
| 4 | `==` `>` `>=` `<` `<=` | Left |
| 5 (lowest) | `&&` `\|\|` | Left |

Note that all bitwise and shift operators share a single precedence level and associate to the right. All comparison operators also share a single precedence level, and `&&` and `||` share a single precedence level. This grouping may differ from other languages, so parentheses should be used when the intended grouping is not obvious.

This table describes the current parser behavior for the operators listed on this page. It does not cover other expression forms.

## Type restrictions

The following restrictions are known from the current type checker:

- The two operands of a binary expression must have a common type.
- The logical operators `&&` and `||` require `bool` operands and produce a `bool`.
- The comparison operators `==`, `>`, `>=`, `<`, and `<=` produce a `bool`.
- The arithmetic operators `+`, `-`, `*`, `/`, and `%`, as well as the bitwise operators `&`, `|`, `^`, `<<`, and `>>`, produce a value of the common operand type.
- Compound assignment operators follow the same operand and result-type rules as their corresponding binary operators. The result must also be assignable to the left-hand side, which must be a mutable place.
- Operator overloading is not supported.

The full set of concrete types accepted by each operator is not yet documented. In particular, this page does not specify whether every arithmetic or bitwise operator is valid for every numeric type.

## Not currently supported

Operator overloading is not supported. Compound assignment is also not available for comparison operators (`==`, `>`, `>=`, `<`, `<=`), since those produce a `bool` rather than updating the left-hand operand. Any binary operator form that is not listed in the summary tables below is not part of the current operator set.

## Summary

### Binary operators

| Operator | Syntax | Category |
|----------|--------|----------|
| `Add` | `+` | Arithmetic |
| `Sub` | `-` | Arithmetic |
| `Star` | `*` | Arithmetic |
| `Slash` | `/` | Arithmetic |
| `Remainder` | `%` | Arithmetic |
| `Equals` | `==` | Comparison |
| `GreaterThan` | `>` | Comparison |
| `GreaterThanOrEqual` | `>=` | Comparison |
| `LessThan` | `<` | Comparison |
| `LessThanOrEqual` | `<=` | Comparison |
| `LogicAnd` | `&&` | Logical |
| `LogicOr` | `\|\|` | Logical |
| `And` | `&` | Bitwise |
| `Or` | `\|` | Bitwise |
| `RightShift` | `>>` | Bitwise shift |
| `LeftShift` | `<<` | Bitwise shift |
| `Xor` | `^` | Bitwise |

### Compound assignment operators

| Syntax | Equivalent to | Category |
|--------|---------------|----------|
| `+=` | `lhs = lhs + rhs` | Arithmetic |
| `-=` | `lhs = lhs - rhs` | Arithmetic |
| `*=` | `lhs = lhs * rhs` | Arithmetic |
| `/=` | `lhs = lhs / rhs` | Arithmetic |
| `%=` | `lhs = lhs % rhs` | Arithmetic |
| `&&=` | `lhs = lhs && rhs` | Logical |
| `\|\|=` | `lhs = lhs \|\| rhs` | Logical |
| `&=` | `lhs = lhs & rhs` | Bitwise |
| `\|=` | `lhs = lhs \| rhs` | Bitwise |
| `^=` | `lhs = lhs ^ rhs` | Bitwise |
| `<<=` | `lhs = lhs << rhs` | Bitwise shift |
| `>>=` | `lhs = lhs >> rhs` | Bitwise shift |
