# Functions

In slynx functions are written such as:

```slx
func main(): void {}
```

Functions are the most basic thing you should know since they are used to create logic.

## Parameters 
```slx
func add(a:int, b:int): int {
  return a + b;
}
```

Functions can receive any amount of parameters and they must be typed as well. The parameter definition on the function is marked by `name: type` separated by commas inside the parenthesis

## Single Expression

Single expression functions are functions whose body is a single expression. Due to so, they can be written with `->` and the expression will be their return.

```slx
func add(a:int, b:int): int -> a + b; //same as example above
```

## Trailing return

Functions on the language are not required to have an explicit 'return' statement. Instead, the last expression will be treated as the return for some. For example:

```slx
func clamp(v:int, min: int, max: int): int {
  let mut out = v;
  if out < min {
    out = v;
  }else if out > max {
    out = max;
  }
  out
}
```
This will implicitly return the value of `out`

## Calling Functions

To call functions the code is simple as `name(arguments)`, for clamp and add provided before it can be written as:

```slx
func main(): void {
  let six = add(5,1);
  let seven = add(4,3);
  let dementia = clamp(six,seven, 69);
}
```
