# Components

In slynx the components are data structures idealized to be usedto handle UI, since the language is made for so.
Components are pretty incomplete because the goal was to stablish features that would make their implementation easier on the userland(interfaces for example).

## Properties and vars

Components contain 2 kind of values, properties and 'vars'. Properties are values idealized to be mutated and when that happens, they update all their dependants automatically. vars on the other hand,
when changing nothing happens, then are no reactive values. They exist cause there might be values that we do not want to emit a rerender when changing, even though they can be used on the calculation
of reactive values. For example:

```slx

component Counter {
  pub prop count: sint32;
  Div {}
}
```
Here the property 'count' is of type 'sint32'(actually 'int' at the moment), since it ain't got no initializers, it's value must be provided on instantiation.
```slx
func main(): void {
  let c = Counter {}; //invalid cause count has no initializer
  let anotherc = Counter {
    count: 0
  };
}
```

To avoid writing the initialization of the properties everytime, the properties are able to receive a default parameter, followed by: '='. 
```slx
component Counter {
  pub prop count: sint32 = 0;
  Div {}
}
```
Now, everytime a `Counter{}` expression appears, it is the same as writing it with `count` property with value 0. 

```slx
component ColorPicker{
  pub prop hue: s.value;
  pub prop s: f32 = 1.0;
  pub prop v: f32 = 1.0;
  prop color: hsl(hue, s,v).toRgb();
  s: Slider<f32> {
    start: 0.0,
    end: 360.0,
  }
  Div {
    style:Colored(color),
  }
}
```

In this example, the idea is pretty simple, we give the default value of hue, or it inherits it from the 'Slider'. The props that are NOT defined with 'pub' are not able to be written nor read from outside the component
defining them itself.
And so, when changing the value of slider, the 'hue' property is updated, with it, the property 'color' and with it, the style of the 'Div' component.

## Styles
Styles on the language are idealized to be an abstraction over function + objects. Since the language is focused on UI in general, making styles such as CSS aint going help when we lead it to an UI where CSS does not exist.
Styles documentation can be found at [here](Styles.md).

## Internal Representation
Differently of react or others, the internal representation of the ui graph is simply no ui graph. The graph only lives during the compile time, and thus all the dependencies are tracked correctly and the minimum amount
of code necessary is really emitted. So there is no vdom, signals, or whatever, there is simply functions.
