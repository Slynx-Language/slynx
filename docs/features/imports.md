# Imports

## Overview

The import system allows a file to use declarations (objects, components, functions, enums, stylesheets) defined in other files or in the standard library. Only declarations marked as `pub` are accessible to other modules (see [visibility-modifiers.md](visibility-modifiers.md)).

## Main Idea

The `import` declaration loads a module — a source file — and optionally selects which names become visible in the current scope.

```slynx
import path.to.module;
import path.to.module using Name;
import path.to.module using { Name1, Name2 as Alias };
```

After the import, the names exported by the module become available.

## Syntax

### Basic import (entire module)

```slynx
import std;
import utils.components;
```

### Selective import (one name)

```slynx
import std using Color;
```

### Selective import (list of names)

```slynx
import utils.components using { Text, Div };
```

### Import with renaming

```slynx
import another using BgGreen as Bg;
import styles using { Background as Bg };
```

## Module Resolution

The import uses dot-separated paths (`a.b.c`). Resolution (`crates/module_loader/`) looks for the module in three roots:

| Prefix | Root |
|---------|------|
| `root.` | Directory of the entry point file |
| `std.` | Directory of the standard library |
| (none) | Directory of the file performing the import |

Importing a directory loads all files within it as sub-modules. Already-loaded files are deduplicated by canonical path.

## Examples

### Module with exports

```slynx
// utils/components.slx
pub component Text {
    pub prop text: str;
}

pub component Div { }

// but...
component Private { }
```

### Usage

```slynx
// main.slx
import utils.components using { Text, Div };

func main(): Component {
    Div {
        Text {
            text: "Oi"
        }
    }
}
```

### Renaming

```slynx
import styles using { Background as Bg };

component App {
    Div {
        style: Bg(0xffffff)
    }
}
```

## Rules

- Only `pub` declarations are accessible to importers.
- Using a non-exported (private) declaration from an imported module is an error.
- `using` is optional; without it, the entire module is imported.
- `using Name` imports one name; `using { A, B }` imports several; `as` renames.
- The semicolon terminates the `import` declaration.

## Interaction

- Parser: `parse_import` in `crates/parser/src/import.rs`.
- The module loader (`crates/module_loader/`) resolves `FileId`, imports sub-modules from folders, deduplicates by canonical path, and looks up exported declarations.
- Import errors (file not found, name not exported) are reported with file context.

## Summary

| Form | Syntax |
|-------|---------|
| Entire module | `import path.to.module;` |
| One name | `import m using Name;` |
| List | `import m using { A, B };` |
| Renaming | `import m using A as B;` |
