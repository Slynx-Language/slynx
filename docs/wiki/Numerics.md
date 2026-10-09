# Number Types

In slynx, since the idea is to be a general programming language at the long run, the language is idealized to have a high amount of numbers to be able to represent bitpack structs and more precise
calculations.
At the moment the language contains only `int` and `float` number types, but the idea is to create signed, unsigned and more float variants.

## Integers
Integers on slynx are idealized to be separated in signed and unsigned, with name distinction being `sint` for `signed int` and `uint` for `unsigned int`. There is no specific length for them, it's then choose by
the suffix that gives it its size. The suffix must be an integer from 0 to 255, so types from `sint0` all over to `sint255` same with `uint0` to `uint255`

### Unsigned

Unsigned numbers are integers that cannot contain negative numbers, so a number such as `uint48` contains 48 bits and them all used to represent only positive numbers.

### Signed

Signed numbers in counterpart, can represent negative numbers, so `sint48` can represent about 2^47 numbers in both directions, -2^47 until 0, and from 0 up to 2^47-1.

#### Internal Representation

The internal representation of these numbers will be internally, the next power of 2 number which is higher than the bit width. For `sint48` and `uint48` both internally are represented with 64bits. This is idealized
so the compiler can make bit packing when possible. The reason is related to memory alignment and how processors read memory.

## Floats

Floats on slynx are not idealized to have an arbitrary bit width, but instead limit for only: `f16`,`f32`,`f64`,`f80`, and `f128`.

## Note
Nothing of this is implemented at the moment. At the moment, only int and float types exist in the userland. The compiler can handle these types already, but it isn't a feature yet due to more important features i think
demand more of my time.
