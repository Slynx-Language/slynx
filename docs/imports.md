# Imports

In Slynx, imports are built around the concept of modules. Every file and every 
folder is a module. Take the following tree as an example:

```
src/
  commands/
  ui/
    elements/
    animations/
      Bouncing.slx
    styles/
  network/
    routers/
    fetchers/
      googleFetcher/
        SomeFetcher.slx
      metaFetcher/
        SomeAnotherFetcher.slx
```

`commands`, `ui`, and `network` are folder modules that contain other modules 
inside them.

## Importing

```slynx
import ui.elements;           // imports all pub things in files directly inside elements/
import ui.animations.Bouncing; // imports Bouncing.slx specifically
import network.fetchers;      // imports googleFetcher and metaFetcher as namespaces
```

The difference between importing a folder module and a file module is:
- **File module** — imports all `pub` symbols from that file flat into scope
- **Folder module** — imports file modules inside it flat, and nested folder modules 
as namespaces

So after `import network.fetchers`, you'd access things like:

```slynx
let fetcher = googleFetcher.SomeFetcher.Fetcher.new();
fetcher.request(googleFetcher.SomeFetcher.UserInfo("john doe"));
```

Because `googleFetcher` is a folder module, it becomes a namespace — its contents 
are not imported flat.

## Relative and Absolute Imports

```slynx
root.ui.elements       // from src/ root
super.animations       // one level up
super.super.commands   // two levels up
```

## Aliases

If two imports would produce a name conflict, you rename with `using`:

```slynx
import commands using ui as uiCommands;
// now ui refers to the top-level ui/, and uiCommands refers to commands/ui/
```

## Selective Imports

```slynx
import ui.elements using {Button, Card};             // only Button and Card
import ui.elements using {Button as Btn, Card};      // with rename
```

## Visibility

Everything is private by default. Use `pub` to expose:

```slynx
pub object Button { ... }
pub func new(): Button { ... }
pub let version = "1.0";
```

## Edge Cases

- A file and a folder with the same name in the same directory is a compile error.
- Circular imports are allowed — the compiler resolves them via a two-pass 
  collection before compiling anything.
