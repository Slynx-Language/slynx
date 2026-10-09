# Enums

Enums in slynx are practically a copy of rust enums, so to write some you can do such as:

```slx
enum Option {
  Some(Person),
  None
}
func main(): void {
  let a = Person(...);
  let myoption = Option.Some(a);
}
```
In case these are being named as 'associated values', so `Person` is an associated type to this `Option` type

## Matching Variants
Even though at the moment its not 100% functional, to analyze the variant of some enum there is a 'matches' expression that helps in so. For example:

```slx
func main(): void {
  let myoption = someoption();
  if myoption matches Some(4) {
    
  }
}
```
Checks if the variant of 'myoption' is Some, and the inner valeu is 4. At the moment there is no support for bindings such as `if myoption matches Some(x) && x > 4` but its idealized in a future where pattern matching is properly
implemented.
The value after the 'matches' expected is the name of the variant and internally the values. 

## Object Variants 

Object variants are associated values but instead of naming another type to be inside, the body of the value is declared inside the variant. 

```slx
enum DrawCommand {
  Rectangle {
    size: Vec2,
    position: Vec2
  }
}
```

## Designed features
In the future, it's idealized to implement a better 'matches' implementation to handle things such as 
```slx
if drawcmd matches Rectangle {size, position} && (size.x < 0 || size.y < 0) || !canvas.rectangle.contains(position) {
  dothing;
}
```
and not only limit 'matches' to enum values, but to desugar values in general as well. 

More about it can be found at [Pattern Matching](idkthepath).
