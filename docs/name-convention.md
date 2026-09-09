# Name Convention

## Types
Every type must follow PascalCase. The unique exceptions are builtin types:
```slynx
object Person {
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
  let udpConnection = UdpConnection();
}
```

## Functions
Functions must follow camelCase and initialize with an action:
```slynx
func addInt(a: int, b: int): int { a + b }
func sendRpc(socket: UdpSocket, data: []int) {
  // ...
}
```
Vectors are `[]T` and fixed-size arrays are `[N]T`.

## Constants
Constants must follow UPPER_SNAKE_CASE:

```slynx
static MAX_COUNT: int = 12;
```
`static` declarations are already implemented (see [features/static-variables.md](features/static-variables.md)).

## Components
Component names follow PascalCase, and properties camelCase.

## Enums
Enum names follow PascalCase, and variants follow PascalCase as well:

```slynx
enum Status {
    Ok(str),
    Failed(str)
}
```

## Styles
Stylesheets follow PascalCase and should be clear on what they'd do.

```slynx
stylesheet Rounded(radius: int) uses Bordered() {
    styles {
        default {
            border_radius: radius
        }
    }
}
```
Style property names inside `styles { }` follow camelCase.

## Files
Files must follow the same convention as the declarations they contain (PascalCase for
types/components, camelCase for functions). Imports are implemented; see
[features/imports.md](features/imports.md).