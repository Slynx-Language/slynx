# Components

In slynx, components are object-like things that help the generation of reactive code. The primary goal
of a component is to make it easier to write UI. For so, it contains properties(might change on the future)
that are reactive, thus, update their dependents on their changing.

## Declaration

To define a new component you can write it such as:
```slx
component MyComponent {
  ///here goes all the things
}
```

### Properties

In slynx, at the moment, the way to define a property is by using the 'prop' keyword. The 'prop' is intended to be
a reactive state. A prop can have a default param, and have its value passed from the parent. For example:

```slx
component MyComponent {
  prop myproperty: sint32 = 0;
  pub prop mysecond: sint32;
  pub prop mythird: sint32 = 12;
}
```

The difference between the 3 is that: the first "myproperty" is not public, and thus, cannot be either accessed
or have value set by the component owner. The "mysecond" is public and does NOT have a default parameter, and thus
MUST contain a value provided by the owner. And "mythird" even though is able to receive a value and be read from
an owner, if the owner does not provide a value to it, its default value is simply 12.

## Children
```slx
component MyComponent {
  Text {
    text: "Hello World"
  }
}
```
The components are able to define children. When it is like the first one, the child is not named, and cannot
be referenced by the component, in case, for it to be referenced, it must contain a 'name=Child{}' syntax like the following:

```slx
component MyComponent {
  pub prop myText = t.text;
  t=Text {
    text: "Hello World"
  }
}
```

By making the usage of `t=Child{}` we can get a reference to this Child component via the name 't'.

## Methods
Components may also contain methods. This makes it easier to provide apis and write code logic on the UI
without the need of workarounds. The methods syntax is the same as the methods on objects:
```slx
component MyComponent {
  pub prop myText = t.text;
  t=Text {
    text: "Hello World"
  }
  pub func printText(&self): void -> print(self.myText); //self.t.text also works
  pub func appendText(&mut self, text:str): void {
    self.myText.append(text);
  }
}
```

## Callbacks
The ability of defining callbacks for specific properties is a thing that might help the language on the long
run. For example, lets say I'm writing a component for the web. I might want a `Text` component to be like the following:

```
component Text {
  pub prop text: str;
  prop element = document.createElement("p");
}
```
But how to, on changing the value of `text`, update the value of `element`? The solution for this is callbacks.
The goal is to make `str` expose a possibility of callback such as:
```slx
object str {
  callback onUpdate(str): void after set(value) => (value);
  pub func set(&mut self, value:str): void {
    *self = value;
  }
}
```

And then everytime a `t.text.set(mystring)` occurs, the defined callback 'onUpdate' is called. Note that the
callback must be provided by the component, and it only exists on component contexts.
The `Text` component could be defined the following way:

```slx
component Text {
  pub prop text: str {
    pub func onUpdate(&mut self, value: str): void {
      element.textContent = value;
    }
  };
  prop element = document.createElement("p");
}
```

## Future Changes
At the moment the properties system is actually pretty limited. Since the goal is to make the reactivity work
with no 2 way binding, this property system gets pretty limited. For example a component that gets only the first
5 letters of a property, this would be implement such as:
```slx
component Substring {
  prop substr = t.text.substring(0..5);
  t=Text{};
}```

The main problem is 'what if i change substr directly and later the t.text?' This is a problem that I got
no awnser but to change how properties work.

The goal is to differentiate between 3 types of properties:

* One whose value is simply inserted on the component and cannot be mutated. This one is intended to be used
when this dependency is tracked by the owner instead. For example a `UsersList` component that might receive
a vector of Users. The idea is that the list do NOT change the list directly. And so the list is managed by
the owner. Reactivity work most on this one

* One which value is mutated inside the component, and when a value is provided, it is just during instantiation,
which leads to no 2 way binding, and an owner whose value is reactive passes this property its value do not create a
reactive binding at all.

* One to be simply views over other properties. It doesn't need necessarily to exist on the physical layout
of the component, but instead be a way to access an specific component operation, which can yes be reimplemented
using a simple method.

Note that reactivity might be a mix of the first and the second properties. The first one should be only for
input, and when the value provided comes from one of the second category, it simply updates it, since it cannot
be mutated internally. 

Other note that must be noticed is that the third one, the views, might not actually be a reactive thing, but
instead simply a syntax sugar for methods, for example, suppose the following(not 100% sure if the syntax will
be this one):
```slx
component Example {
  prop a: sint32 = 10;

  view b = a * 2;
  view c = b + 5;
  view d = c * 3;
}```

This can safely be rewritten as:

```slx
component Example {
  prop a: sint32 = 10;

  pub func viewb(&self): sint32 -> self.a * 2;
  pub func viewc(&self): sint32 -> self.viewb() + 5;
  pub func viewd(&self): sint32 -> self.viewc() * 3;
}
```
The only problem is is a view is expensive. If so it'd need to be a field on its own, instead of a syntax
sugar for functions, but this might be a thing on the future.
