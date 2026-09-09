# Stylesheets

Stylesheets on the language are a Non CSS inspired way to define styles. They are at the moment not fully implemented on the codebase, due to lacking of features to make them lower properly to what is intended. 
They are idealized to be somewhat like pure functions that return an style, and thus can be applied to components.

## Definition
A stylesheet can be defined by the following(even though things might not be available yet such as '+=' operator):

```slynx
stylesheet Name(arg: int, arg2: Point) uses Other(arg) {
  let mut accum = 0;
  for i in 0..100: accum += i;
  styles {
    backgroundColor: hsv(accum % 360, 1, 1) animating(300ms, ease-in-out),
  }
}
```

The stylesheet can receive arguments and use them as normal params a normal function would have. The main difference from a stylesheet to a function is that a stylesheet MUST contain a 'styles' block at the end telling
what are the values that the style will have. Another difference to a normal function is composition. The stylesheet's got an 'uses' clause, that tells what other styles this one inherits from, in case what styles it uses as well.
The priority goes from left to right, and the current style ALWAYS wins at the final. Thus lets say the given:

```slynx
stylesheet Bordered() {
  styles {
    borderRadius: vec4.flat(12px)
  }
}
stylesheet Bg(color: Rgba) {
  styles {
    backgroundColor: color
  }
}
stylesheet AnotherThingToConflict() {
  styles {
    backgroundColor: green
  }
}

stylesheet MyStyle() uses Bordered(), Bg(red), AnotherThingToConflict() {
  styles {
    borderRadius: vec4.flat(16px),
  }
}
``` 
In this case the output of 'MyStyle' will contain backgroundColor as 'green', and borderRadius as flat 'vec4(16px,16px,16px,16px)'. The reason is simple, bordered sets the border to be 12px, but it gets overriden by the style itself, that
requests 16px. The bg requests background color to be red, but the 'AnotherThingToConflict' requests it to be green, so it wins. And yes the code of them all is executed.

## Animations
Animations on styles are idealized to be pretty idiot, they can be made written in 'groups' or individually. The main thing is that there is no concept of hover, clicked, etc for the styles, the style should never know about these concepts, because what if some backend doesn't support the concept of hover for example? Anyways, for so, we can still use the values from the parameters, for example:

```slynx
stylesshet MyButton(hover: bool) {
  styles {
    backgroundColor: hover ? red : black animating(150ms, ease-in);
    scale: hover? 1.5 : 1.0 animating(150ms, linear);
  }
}
```
this is the definition of individually animation, each property has its animation defined individually. But for cases where the style is applied to a bunch we can group them to use the same flag. For example:
```slynx
stylesheet MyButton(hover: bool) {
  styles {
    backgroundColor: black;
    scale: 1.0;
    when hover animate(300ms, ease-in) {
      scale: 1.5;
      backgroundColor: red;
    }
  }
}
```
This is the same as the one before, the difference is that when 'hover == true' it applies the given styles on the 'hover' block with that given animations.
In case the definition of a block should not specify a requirement for duration nor easing, in case, just the application of some style when that given value, that MUST be DIRECTLY or INDIRECTLY related to some parameter, is true.
For example, the previous code could be rewritten with:
```slynx
stylesheet MyButton(hover: bool) {
  styles {
    backgroundColor: black animating(300ms, ease-in);
    scale: 1.0 animating(300ms, ease-in);
    when hover {
      scale: 1.5;
      backgroundColor: red;
    }
  }
}```
Even though grouping should be preferrable.

## Complex Checks
A style could do a lot of things, for example, checking if it should contain some specific layout, hovering, etc. For example:
```slynx
object CardArgs {
  windowSize: Vec2<px>,
  hover: bool,
  clicked: bool,
}
stylesheet Card(args: CardArgs, theme: Theme){
  styles {
    layout: grid(5,3);
    scale: 1.0;
    shadow-color: transparent;
    shadow-blur: 0px;
    when args.windowSize.width > 720px {
      layout: grid(10, 7);
    }
    when args.hover animate(300ms, ease-in) {
      scale: 1.15;
      shadow-color: purple;
      shadow-blur 12px;
    }
    when args.clicked animate(300ms, linear) {
      scale: 0.9;
    }
  }
}
```
Note that the `args.windowSize.width > 720px` doesn't apply any animation at all, it simply changes the layout from grid(5,3) to grid(10,7) immediatly.

## Generics
Since stylesheets are declarations such as objects, functions, etc, and in fact, they are functions that return a style to be applied, they do yes are able to use generic types, they will be monomorphized and have an implementation for each T used.

### Internals
At the moment this feature is not implemented due to the lack of interfaces to make it possible to write styles in a simple way. To understand how styles are implemented interally read `docs/contributing/styles.md`
