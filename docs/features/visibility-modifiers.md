# Visibility Modifiers

## Overview

Top-level declarations in Slynx can be marked with `pub` to make them visible outside the file where they are defined. Without `pub`, a declaration is private to the file.

## Main Idea

Visibility controls which declarations can be used by other modules/files. By default, everything is private. The `pub` keyword makes a declaration publicly accessible. For components, there are also visibility variants specific to the component tree.

## Syntax

```slynx
pub object Person { ... }
pub component Header { ... }
pub func helper(): int { ... }
pub alias PersonRef = Person;
pub enum Status { ... }
pub stylesheet Theme() { ... }
pub static MAX: int = 100;
```

## Modifiers

| Modifier | Scope | Applicable to |
|----------|-------|---------------|
| `pub` | Visible to all | All declarations |
| (none) | Private to file (default) | All declarations |
| `pub(parent)` | Visible only to the parent component | Component members |
| `pub(child)` | Visible only to child components | Component members |

## Rules

- The default is `Private` — declarations without `pub` are not accessible outside the file.
- `pub(parent)` and `pub(child)` are exclusive to component members (properties and child components).
- `pub(parent)` means only the parent component can access the member.
- `pub(child)` means only child components can access the member.
- Note: enforcement of `pub(parent)`/`pub(child)` is a parser/AST distinction; full semantic checking may not be implemented for all cases.

## Examples

```slynx
// arquivo a.slx
pub object User {
    name: str,
}

object Secret {
    data: int,
}

func main(): void {}
```

```slynx
// file b.slx
import a;

func main(): void {
    // User is accessible because it was declared pub
    let u = User(name: "Ana");

    // Secret is NOT accessible — it is private to a.slx
}
```

## Interaction with components

For component members, beyond `pub`, there are restricted variants:

```slynx
component Parent {
    pub(child) prop data: int;        // visible only to child components
    pub(parent) prop callback: int;   // visible only to the parent component
    pub prop title: str;              // visible to everyone
    Child {}
}

component Child {
    prop parent: int;
}
```

## Summary

| Modifier | Scope | Declarations |
|----------|-------|-------------|
| `pub` | Public | All |
| (default) | Private to file | All |
| `pub(parent)` | Parent component | Component members |
| `pub(child)` | Child components | Component members |
