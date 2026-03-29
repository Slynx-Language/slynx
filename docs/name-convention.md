# Name Convention

## Types
Every type must follow PascalCase. The unique exceptions are builtin types:
```slynx
struct Person {
  age: int,
  name: str
}
component LandingPage {
  pub prop index: float = 0.0;
}
object Socket {
  raw: RawSocket,
  ip: int
}
```
The type must be comprehensible and have a meaning by its own

## Variables
Variable names must follow camelCase. 
```slynx
func f(): int {
  let personAge = 9;
  let personName = "Person";
  let udpConnection = UdpConnection;
}
```

## Functions
Functions must follow camelCase and initialize with an action:
```slynx

func addInt(a:int, b: int): int -> a + b;
func sendRpc(socket: UdpSocket, data: int[]){
  ...
}
```
## Constants
Constants must follow UPPER_SNAKE_CASE:

```slynx
const SOME_VALUE: int = 12; //idealized. Not implemented yet
```

## Components
Component names follow PascalCase, and properties camelCase.

## Styles(idealized)
Styles follow PascalCase and should be clear on what they'd do.

```slynx
stylesheet Rounded(value: size) {
  prop border_radius = value;
}
```

## Files
Must be implemented in future when imports are idealized.
