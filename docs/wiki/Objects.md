# Objects

In slynx there are no classes, instead, a more 'struct' like approach are 'objects'. Even though the name might make you remember of OOP, it idealized to be related to structs.
To write them it's as simple as:
```slx
object Person {
  name: str,
  age: int
}
``` 

And to instanciate it, it's with:

```slx
func main(): void {
  let jhon = Person(name: "jhon", age: 44);
}
```

## Methods

Objects accept methost such as OOP as well.

```slx
object Person {
  name: str,
  age: int,
  pub func new(name: str, age: int): Self {
    Self(name: name, age: age)
  }
  pub func haveAChild(&self): Person {
    Person(name: "mychild", age: 0)
  }
}

func main():void {
  let jhon = Person.new("jhon", 44);
  let child = jhon.haveAChild();
}
```

Every method denoted with `self` at the first parameter is a method that is executed on instanciation, and every that dont, are 'static'.
