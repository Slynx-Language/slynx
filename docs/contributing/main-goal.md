# Slynx Main Goal

The main goal of the language at the mid run is to give the enough features so it can be properly compiled to the browser, desktop etc. This will include things like generics, interfaces, and move semantics. 

## Why these things?

These things are important to have in the language to make the code creation easier and avoid making the compiler having unecessary stuff.
For example, the creation of styles and their usage in components: to make such thing happen easily and without adding too much boilerplate on the compiler IT IS A MUST, to have interfaces/traits.
The main reason for so its due to a style not being a thing that we actually care when defining a component, we simply care that it is a style, and defines how our components should look, due to this, the language needs to have
something that gives the user the hability to simply don't care about the specific type, only how it behaves.

### Remaining features

The remaining features to make the slynx a bit stable are:
* interfaces
* components with reactivity
* styles
* generics(already being implemented)
* move semantics

## Styles refactor
The styles are being marked, even though they were previously implemented, due to a change on how they will work.
In case styles were being implemented and lowered fully on the codegen, to make it easier, now stylesheets are being idealized to be a lowering after the parsing. Thus a style will compile down to struct + a function.
The struct is simply a struct that contains information about the style being implemented, but, with the `Style` interface implementation. So, something such as:

```
stylesheet Colored(bg: Color, fg: Color) uses Bordered(12px) {
    styles {
        backgroundColor: bg;
        foregroundColor: fg;
    }
}
```
is transformed at the HIR level into
```
struct Colored:Style {
    ...borderedFields
    backgroundColor: Color;
    foregroundColor: Color;
    pub fn new(bg: Color, fg: Color): Self {
        let bordered = Bordered::new(12px);
        Self {
            ...bordered,
            backgroundColor: bg,
            foregroundColor: fg,
        }
    }
    pub fn applyOnComponent(self, c:StyledComponent) {
      c.style.applyBackground(self.backgroundColor);
      c.style.applyForeground(self.foregroundColor);
      c.style.applyBorder(self.border);
    }
}
```

and a simple 'Colored(red, blue)' in fact is the same as 'Colored::new(red, blue)'. This will make things easy as hell, since we just need to support interfaces and lowering. So no more declarations or whatever for the codegen. Thus the same for components.

## Components styling 

Components are differentiated between 2 types: Components and StyledComponents. StyledComponents are components that simply implement the `StyledComponent` interface.
In fact this is more low level, for example, to implement a div, it should contain a styles object during runtime, so we can make a component Div  that wraps the runtime div from JS, such as:

component Div {
    inner: div
}

extend Div: StyledComponent {
    func applyBackground(self, color: Color) {
        self.inner.style.backgroundColor = color;
    }
}

and now the component Div supports backgrounds in slynx. Yaay. The usage of generics then would make it simpler, since the lang is idealized to have inheritance via composition. In case a component Div could simply be a `Wrapper<ActualDiv>`
and we simply implement

extend<T> Wrapper<T>: StyledComponent {
    func applyBackground(self, color: Color) {
        self.inner.style.backgroundColor = color;
    }
}

and now any component that wraps an actual div can be styled via `applyBackground`.

### Inheritance
Since inheritance was mentioned, I gotta think how i imagine it. The idea is pretty simple:

struct T extends K {}
can simply be implemented as
struct T {
    super: K
}

and thats it. Now we can safely execute (&T) as &K because the layout of T contains K and more things, so, we can reinterpret as K, to simply ignore the other K. Then a component such as
```
component Div extends Wrapper<div> {}
```
is idealized to be a valid thing, since it will simply be the same as:
```
component Div {
    super: Wrapper<div>
}
```
