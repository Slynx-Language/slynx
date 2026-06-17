# Global Values

Global values on the IR are defined with 'global' before its name, type and value. They are values specific that live somewhere on the code, but as long as the code is running.

When compiling these to binary for example, it might be inserted at .bss section, and when compiling to JS, simply a variable that lives for ever.

Global values are on the Frontend of slynx, immutable to avoid memory corruption, but here on the IR instead they are mutable and so, able to get memory corruption. The idea here is to don't limit what a frontend can/cannot do. Slynx choosed to be safe,
another frontend might choose to not.
```sir
global %value: uint8 = 0;
```
Where to access it on the IR is represented simply by `%value` but for backends, this is a global value pointer, which points specifically for that given value.
