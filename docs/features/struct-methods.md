## Lowering

Struct methods are lowered into ordinary functions by making the receiver an explicit first parameter. The receiver keeps the same ownership semantics it had in the method declaration: `&self` becomes `self: &T`, `&mut self` becomes `self: &mut T`, and `self` becomes `self: T`.

For example, given:

```slynx
pub object Rgba {
    pub fn white(): Self {
        Self.new(255, 255, 255, 255)
    }

    pub fn new(r: u8, g: u8, b: u8, a: u8): Self {
        Self(r: r, g: g, b: b, a: a)
    }

    pub fn negative(&self): Self {
        let white = Rgba.white();
        white.minus(*self)
    }

    pub fn minus(&self, other: Self): Self {
        Self(
            self.r - other.r,
            self.g - other.g,
            self.b - other.b,
            self.a - other.a
        )
    }
}
```

The instance methods can be lowered to functions where the receiver becomes the first parameter:

```slynx
pub fn negative(self: &Rgba): Rgba {
    let white = white();
    minus(&white, *self)
}

pub fn minus(self: &Rgba, other: Rgba): Rgba {
    Rgba(
        self.r - other.r,
        self.g - other.g,
        self.b - other.b,
        self.a - other.a
    )
}
```

The type-associated methods `white` and `new` do not have a receiver, so they are simply functions associated with `Rgba` and do not require an instance parameter.

Method calls are lowered in the same way. `color.negative()` can be understood as `negative(&color)`, while `color.minus(other)` can be understood as `minus(&color, other)`. Likewise, `Rgba.white()` can be resolved to the `white` function associated with `Rgba`.

The important part is that methods do not require a special runtime representation. They are a language-level abstraction that is lowered into ordinary functions, with the receiver becoming an explicit parameter.
