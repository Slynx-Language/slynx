# Generics

There are generics on slynx, every type declaration(any declaration that represents a type, such as objects) is able to contain these. The generic syntax is defined with `<T, K, U,...>`(there are no variadic yet, so these '...' mean
only that it might contain more generics). So for example an option type could be implemented such as:

```slx
enum Option<T>(u8) {
  Some(T),
  None
}
```

The same happens with objects, components, and functions, for example:
```slx
func doAnything<X>(x: X) {...}
```

## Interfaces
In slynx interfaces are a method to tell that a given type T MUST contain the given methods. This is mainly useful when using generics to be able to receive any type that does something. 
For example:

```slx
interface Requester {
  pub func request(&self, target: str): HTTPResponse;
}
func getUser<R>(r: R): HTTPResponse where R: Requester -> r.request("https://url.getUser");
```

In here any type that implementes the interface `Requester` MUST contain the method request with the given signature. 
To implement the interface for the type there are 2 ways:
  1. Write the interface name after the declaration type
  2. Extend the type to implement that interface.

```slx

pub object JSRequester: Requester {
  pub func request(&self, target:str): HTTPResponse -> fetch(target);
}

```
This creates the object JSRequester that implements the interface Requester. This could be extended to implement more interfaces such as `object JSRequester: Requester, AnotherInterface, AThirdOne {}`

## Extensions
Extensions are a method to extend what the type can do without requiring it to be defined within the type. For example:


```slx

interface Lerpeable {
  pub func lerp(self, b: Self, t: f32): Self;
}
extend f32: Lerpeable {
  func lerp(self, b:Self, t:f32): Self -> ...; //i do not remember lerp function
}
```

This extends the type 'f32' to implement the 'Lerpeable' method. This could also extend f32 to implement more interfaces, separating them by ',', thus a code such as: `extend f32: Lerpeable, Clampeable {}`

Extensions do not require an interface to extend the type, so they can simply extend the type and give them new methods, such as

```slx
extend f32 {
  func intoSint32(self): sint32 -> ...; 
}

func main(): void {
  let a: f32 = 44.0;
  let b = a.intoSint32(); //accepted normally
}
```
