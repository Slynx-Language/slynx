# Generic Stylesheets

## Overview

The parser and HIR accept stylesheets with type parameters in the declaration, but **monomorphization of generic stylesheets is not yet supported**: attempting to use a generic stylesheet (`stylesheet A<int>`) triggers `unimplemented!()` in the monomorphizer.

## Main Idea

Declaration:

```slynx
stylesheet A<T>(x: T) {
    styles {}
}
```

The type parameter is read and stored in the HIR (`HirStylesheetDeclaration.generics`), but specialization by type argument at the usage site does not exist yet.

## Syntax

```slynx
stylesheet Name<T, U, ...>(args) uses Parents {
    styles { ... }
}
```

## Rules

- Declarations with type parameters are parsed normally (same syntax as other generics).
- **Using** a generic stylesheet is not supported: the monomorphizer aborts with `unimplemented!()` (`assert_no_generic_non_functions` in `crates/monomorphizer/src/lib.rs`).
- The example `examples/generics/generic_style.slx` compiles because the generic stylesheet is only queued in the HIR when it is *used*; declarations that are never referenced do not trigger the error.

## Examples

```slynx
// Declaring is parsed and accepted, but using `A<int>` does not work yet.
stylesheet A<T>(x: T) {
    styles {}
}
```

## Current Limitations

- Monomorphization of generic stylesheets is not implemented (hard error `unimplemented!()`).
- Inheritance via `uses` with type arguments is also not supported.

## Interaction

- The parser parses the generic declaration with the extra parameter list.
- The HIR records the parameters in `HirStylesheetDeclaration.generics`.
- The monomorphizer rejects any queued generic stylesheet (`assert_no_generic_non_functions`).

## Summary

- Generic stylesheet declarations are parsed: `stylesheet A<T>(x: T)`.
- Specialization via monomorphization is **not** implemented (panics with `unimplemented!()`).
- Generic stylesheets only work as declared dead code that is never used.
