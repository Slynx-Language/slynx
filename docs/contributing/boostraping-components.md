# Bootstrapped Components

Components in the language can be **bootstrapped** by the target platform. This design achieves several important goals:

- Keeps the compiler core small and portable.
- Shifts platform-specific responsibilities (rendering, DOM/scene management, styling) to the target implementation.
- Maintains a clean, declarative component syntax for users.
- Enables high-performance code generation through specialization and monomorphization where beneficial.

The mechanism builds on the existing lang item system (via the `@builtin` attribute) but does **not** require new lang items for every component.

## Component Contract

Every bootstrapped component must provide:

1. A handle to the platform-native object used by the renderer.
2. An `onMount` function (called right after instantiation).
3. A `render` method.

Because components are **views** rather than owning containers, they must insert themselves into their parent during mounting.

### Example

```slx
component Title {
  Text { text: "Jorge" }
}

component Text {
  pub text: str;
  style: StyleContext;

  func onMount(sceneContext: &mut SceneContext) {
    // Called once the component is created and parented
  }

  func render(&self, context: &DrawingContext) {
    context.render_text(self.text, self.style.paint);
  }
}
```

`SceneContext` and `StyleContext` are provided by the target for each compilation.

## Contexts

### SceneContext
- Contains scene-specific data (e.g. reference to the window, root node, event system).
- Holds a reference to the **concrete owner** in the scene tree/graph (not just a wrapper).
- Parent propagation works through the component hierarchy:
  - In HTML: `document.body`
  - In Skia: the window / root canvas
- When a component has no children container, its children are inserted directly into the current parent (command-based model).

### StyleContext
Implements a styling interface that allows declarative styles to be applied efficiently:

```slx
interface Styled {
  func applyBackgroundColor(&mut self, color: Color): void;
  func applyBorderRadius(&mut self, size: Pixel): void;
  // ... other style methods
}

struct StyleContext: Styled {
  paint: Paint;

  func applyBackgroundColor(&mut self, color: Color) {
    self.paint.setColor(color);
  }
  // ...
}
```

When the user writes:

```slx
Text {
  style: SomeStyle(backgroundColor: red, borderRadius: 8px)
}
```

The compiler generates calls to the appropriate `apply*` methods on the `StyleContext`. This keeps styling flexible per target without compiler knowledge of every property.

## Slots

Slots act as typed gaps that parent components can fill at instantiation time. They behave similarly to monomorphization for specific types.

```slx
component WebPage {
  title: Slot<Text>;
  body: Slot;                    // accepts any component
  next: Slot<Button>;
  picker: Slot<dyn NumericPicker>;

  Div {
    Div {
      slot(title)
      slot(picker)
      slot(next)
    }
    slot(body)
    Footer {
      slot(title)
    }
  }
}
```

Usage:

```slx
WebPage {
  title: Text { ... },
  body: Div { ... },
  next: Button { onClick: _ { print("Hello"); } },
  picker: Slider<f32>
}
```

- Specific slots (`Slot<Text>`) are monomorphized only when generics/traits are involved.
- `Slot` (unbounded) and default children use `Vec<dyn Component>` — no monomorphization penalty.
- A new specialized type is generated for each unique instantiation.

## Components with Children

Components that accept children declare a **default slot**:

```slx
component Div {
  inner: DomNode;
  children: default Slot;        // reserved name
}
```

- `children: default Slot<T>` restricts children to type `T`.
- `children: default Slot` accepts any component (`Vec<dyn Component>`).
- Children are inserted via the command-based model (children talk to parents via commands; parents own children directly).

## Desugaring & Lifecycle

`Component { prop: value }` desugars roughly to:

```slx
let context = current_context();
let component = Component { prop: value };
component.mount(context);
```

- **No `update` / `shouldUpdate`**: The language builds the UI graph at compile time and generates specialized code.
- **No explicit `onUnmount`**: Affine types + move semantics + `drop` (similar to Rust) handle cleanup automatically.
- Callbacks for changes can still be added (they run after mutation).

## Retained Mode via Commands

The system is **fully retained**:
- Parents own children directly.
- Children interact with parents through commands.
- This allows efficient updates while maintaining a clean ownership model.

---

This design keeps the language expressive and the compiler lean, while giving targets full control over performance-critical details.
