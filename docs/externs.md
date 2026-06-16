# Externs

Extern values are values/types that do not exist on a slynx code, thus, it might be to reference something that exists on the runtime being compiled, or that came from a library via dynamic/static linking

In the language until now 16/6/2026, there is only 'object' to create structs, but they are idealized to be deprecated later and only able to be created inside externs and give place to 'struct'. The main reason for this is that structs are positional based, and objects will be rewritten to be name based. Name based are idealized only for extern values where the runtime being compiled to is name based, such as JS. 

```syx
extern {
  object Name {
    a: int,
    b: int
  }
  struct Name2 {
    a: int,
    b: int
  }
  static name: Name;
  func f(a:Name): void;
}
```

The main difference is that 'Name' is name based and so it uses the name of the fields to access rather than its position. Outside extern blocks they are not permitted and will give a parse error. 

Externs do not specify where these things come from, just specify that they exist, so they are more sensible, because if names are written wrongly, and if on a struct, fields are positioned wrongly as well, then it might lead to runtime bugs.

Later externs will be implemented to also consider the 'capability' the compiler has. For example
```syx
extern "js" {
  object Document {
    func createElement(tag:str):DOMElement;
  }
  object DOMElement {};
}```
Where this extern is only parsed if the compiler has a capability named 'js', which will be implemented later. By know, think of it such as rust features.
