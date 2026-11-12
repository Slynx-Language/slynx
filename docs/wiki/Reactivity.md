# Reactivity System

The reactivity system works mainly to avoid bugs and help the creation of code that doesn't require any sort of algorithm to get track of dependencies during runtime. The goal is to then, make all these
reactivity systems a compile time thing, only emitting what is really required for the code run the faster it can.

For this, the reactivity system is separated in: properties, inputs and views.

## Properties

Properties on the code are being intended to be values that a component owns and does not represent no binding with any other property. Thus if a parent provides a value to a child such as:

```slx
component Parent {
  pub prop myvalue: sint32;
  Child {
    childrenValue: myvalue;
  }
}
```

The `childrenValue`, if a property as well, does not make any reactive binding, instead, simply receives the initial value of 'myvalue' by copy and now it owns that copy. When 'myvalue' changes no updates goes to 'childrenValue'
This is a thing because parents should NOT be able to have their values updated for a child at all. This evicts a parent to provide a value and a child component change it without you want.

## Views
Views are intended to be a way to get a computation derived from 1 or more properties. Since its a 'view' it must not have any kind of mutation.
For example:

```slx
component ColorSelector {
  pub view color: Rgba = hsv(hue.value, saturation.value, brightness.value);
  hue=Slider{ min: 0, max: 360, value: 180, rate: 0.05}
  saturation=Slider{min: 0, max: 1.0, rate: 0.01}
  brightness=Slider{min: 0, max: 1.0, rate: 0.01}
}```

The `color` is now a field on the component and no reactivity is tracked at all(automatic updates, unless color is used in a local where a binding happens). In fact that `color` view is intended to be an abstraction over:
```slx
component ColorSelector {
  pub func color(&self): Rgba -> hsv(hue.value, saturation.value, brightness.value);
  hue=Slider{ min: 0, max: 360, value: 180, rate: 0.05}
  saturation=Slider{min: 0, max: 1.0, rate: 0.01}
  brightness=Slider{min: 0, max: 1.0, rate: 0.01}
}```

So, even though the color is not a field that changes constantly, it can be accessed in a way that it is always updated.

## Inputs

Inputs are values that are provided by a parent, but that the receiver cannot change at all. This is intended to avoid 2 way binding. Since the inputs mainly come from a parent property, when this parent property updates
it updates as well this 'input'. For example:

```slx
component UserList {
  pub prop myUsers: Vec<User>;
  VerticalUsers {
    users: myusers
  }
}
```

Even though `VerticalUsers` can access the `myusers` it cannot mutate it, because it's a responsability of the `UserList` only. And then when it updates the `myusers`, then the VerticalUsers re-renders.
The same can yes work with views, even though not necessarily at the moment to actually initilize the reactive system focusing mainly on `prop` -> `input` reactivity.

### Possible problems

The reactivity with views I think is a bit trickier, since it'll must be able to notice when a child property updates, and then emit that it requires to update on the usage of that view. For example:

```

component ColorSelector {
  pub view color: Rgba = hsv(hue.value, saturation.value, brightness.value);
  hue=Slider{ min: 0, max: 360, value: 180, rate: 0.05}
  saturation=Slider{min: 0, max: 1.0, rate: 0.01}
  brightness=Slider{min: 0, max: 1.0, rate: 0.01}

  Text {
    text: "Hello World",
    style: Fg(color)
  }
}
```

The color updates everytime hue.value, or saturation.value or brightness.value change, but if that `value` is another view, it must track where that come from. This really leads to only 2 things: properties and inputs, but the main problem
is how to know when they change and how to express this on a graph during compile time


## Observations

Even though the goal is to be even more agressive than svelte for example related to how updates are made and generated, there are, for sure, things that will end up by needing a minimum amount of runtime reactivity.

The goal is not to avoid literally every kind of reactivity system on runtime, but, anything that is known during compile time, must be resolved, and the ones missing on runtime should actually be implemented in a idiot way.

In a idiot way because the more simpler it is, faster it is. A component that depends on a condition to exist could be simply implemented by using tagged unions. And thats it.

In a short sentence: the goal is to avoid unecessary abstractions
