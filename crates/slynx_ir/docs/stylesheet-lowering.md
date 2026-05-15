# Stylesheet Lowering Guide

This document describes how the `stylesheet` construct in Slynx surface syntax is lowered into the SlynxIR.

## Overview

A `stylesheet` in Slynx mixes three distinct concepts, each of which maps to a different IR construct:

1. **Styling** — the style properties themselves → lowered to a **struct**
2. **Turing completeness** — expressions, `let` bindings, parameters → lowered to a **function**
3. **Reuse / inheritance** — the `uses` keyword → lowered to a **struct merge**

## Basic Lowering

Something like the following:

```slynx
stylesheet Rounded(size: px) {
  styles {
    borderRadius: size
  }
}
stylesheet RoundedRed() uses Rounded(5px) {
  styles {
    backgroundColor: red
  }
}

component C {
  Div {
    style: Rounded(12px),
  }
}
```

should generate the following:

```slynxir
struct RoundedStyle {i32}; // a pixel is a 'i32' internally
struct RoundedRed{i32,i32}

%RoundedRed GenerateRoundedRed(){
$entry:
  %v = %RoundedRet{5px, 0xff0000};
  ret %v;
}

void ApplyRoundedRedStyle(PrimitiveComponent, RoundedRed) {
$entry:
  roundness = get_prop p1, 0;
  @sapply BorderRadius, p0, roundness;
  bg = get_prop p1, 1;
  @sapplu BackgrounColor, p0, bg;
}

void ApplyRoundedStyle(PrimitiveComponent, RoundedStyle) {
$entry:
  roundness = get_prop p1, 0;
  @sapply BorderRadius, p0, roundness;
  ret;
}

component %C() {
  #t0 = specialized Div;
  @initcall ApplyRoundedStyle, #t0, %RoundedStyle{0, 0xff0000}
}
```

The stylesheet becomes two things:
- A **constructor function** (`Rounded`) that builds the style struct from parameters
- An **apply function** (`ApplyRoundedStyle`) that reads the struct fields and calls `@sapply` for each property

The component uses `@initcall` to apply the style before the component is inserted into the UI tree.

## Lowering of `uses` (Inheritance)

The lowering of `uses` should create a struct which contains the style properties of all referenced stylesheets merged together. For example:

```slynx
stylesheet A() {
  styles {
    borderRadius: 7px
  }
}

stylesheet B() uses A() {
  styles {
    backgroundColor: Blue
  }
}
```

The `B` style could be represented as the following:
```
struct A {
  borderRadius: i32
}

struct BIntermediate {
  backgroundColor: i32,
}

struct B = merge(A, BIntermediate);
```
note: the given code is a simple example to exemplify, internally only A and B will exist and no `merge` function exists.

Where `merge` should **not copy fields** — if `A` has the same fields as `BIntermediate`, it uses the ones from `A` and ignores the ones from `BIntermediate`.

## Precedence Rules

The `uses` clause follows left-to-right precedence:

```slynx
stylesheet C() uses A(), B() { ... }
// precedence: A < B < C
```

The apply function for `B` should have a single `@sapply` operation per style property. Properties with lower precedence get their `@sapply` called first, and properties with higher precedence override them. If `A` and `B` define the same property, the `@sapply` for `B`'s value is used.

## Field Ordering Convention

Style struct fields follow a fixed positional order. Base style properties come first, states come after. The backend identifies each property by its position index, not by name.

The full list of positional style properties and their numeric codes is defined in [STYLES_TABLE.md](./STYLES_TABLE.md).
