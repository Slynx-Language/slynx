# Slynx IR

Slynx IR specifies what the backend compiling it should do instead of how, even though the language is a bit opinionated about how to do so.
First of all the IR follows SSA and is extremely typed.
The IR has got the concept of 'contexts' that are anything able to be run, this means a struct isn't a context, because it by itself cannot execute code, but a method or a function are, as well as components that can have code
to be run, such as the reactivity model.

## Syntax

### Basic Types

Primitive scalar types used by this IR:

* Signed integers: `i8`, `i16`, `i32`, `i64`, `i128`
* Unsigned integers: `u8`, `u16`, `u32`, `u64`, `u128`
* Floating point: `f32`, `f64`
* Boolean: `bool`
* Pointer-sized unsigned integer: `usize`, `isize`

`usize` means target-native pointer-sized unsigned integer when the backend supports it.
If a backend has no native `usize` representation, it must treat `usize` as `u64`.
The same idea is used for `isize`, if the backend has no isize representation, then it's represented as a `i64`

Text and byte-oriented primitive types:

* `str`: immutable UTF-8 string handle type
* `bytes`: immutable byte-sequence handle type

Language-level aliases currently used in examples:

* `int` maps to an integer primitive selected by the frontend/lowering stage which is idealized to be i32
* `float` maps to a floating primitive selected by the frontend/lowering stage which is idealized to be f32


### Structs
The IR has implementation of operations primitives. The syntax for the deffinition of a struct can be the following:

`struct %S {int, float};`

Which represents the following struct:

```slynx
object S {
  property_1: int,
  property_2: float
}
```

Tuples(WIP) use the same method, so a tuple in slynx denotated by
```slynx
object T(int,float);
```

is defined by the same way as `S`, the only thing is how their fields are accessed on the code. (Tuples must be implemented in slynx yet)

### Strings
Strings on the IR are represented as internalized values. On the IR they live on a separated struct called Internalizer, and their access can be made via slices. Note that this IR expects them to be UTF8, and be represented as, inside the IR

```slynxir
struct %StrHandle {usize, usize}
```
which in slynx would be on slynx the equivalent to

```slynx
struct StrHandle {
  ptr: usize,
  length: usize, //length in bytes
}
```

Note that this is a internal struct that is automatically put on the IR.
When referencing a string, what the IR will see it's the string logical pointer, so the specialized backend only sees it as well, and based on this pointer, the compiler can access the string on the Internalizer.
Strings or `str` type in slynx, are supposed to be utf8 and immutable. Due to that, on the IR textual form, the way of seeing the usage of a string is via %StrHandle.

Suppose the following:
```slynx
object Person {
  name: str,
  age: int
}
func main(): Person{
  let p = Person(name: "jorge", age: 44);
  p
}
```

on the IR it would be represented as
```slynxir
struct %StrHandle {usize, usize}
@str0 = "jorge"; //this is a handle to the string "jorge", but its just for readability, because internally its the %StrHandle

struct %Person {%StrHandle, i32}

%Person main(void) {
  p = %Person{@str0, 44};
  ret p;
}
```

and so the compiler can determine how it should be done internally.


### Contexts
All 'contexts' are referred as any piece of code that executes some sort of code. A struct cannot be considered a context by itself because it cannot execute any sort of code, but can be used for execution.
Based on that, all contexts are composed by basic blocks.
Basic Blocks have a linear sequence of instructions and MUST terminate with some termination operation. Since its linear there are no jumps.
Note that termination operations do not terminate the context, but rather, the current block, which means that we can enter another block via `labels`

#### Labels
Labels are named as `$name` and represent another block that can be used run. All labels are meant to be declared inside a context and be used only by that specific context. Think of them like:
```slynxir
i32 main(i32){
$entry:
  inc = addi32 p0, 1;
  br $end
$end:
  twice = muli32, inc, 2;
  ret twice;
}
```

Since a label is a named block, it needs to terminate with a termination operation.

#### Functions
Functions are defined on the IR level as the following:
```slynxir
i32 #add(i32, i32) {
$entry:
  result = addi32 p0, p1;
  ret result;
}
```
Where p0 is the first parameter and p1 the second one. The order of the names then is pN where N is the (N+1)th parameter of the function.
Something in slynx such as
```slynx
struct Currency {
  money: int
}

func currency_of(money: int): Currency {
  Currency(money: money)
}
```
in the IR can be represented by

```slynxir
struct %Currency {i32}

%Currency currency_of(i32) {
$entry:
  result = %Currency{0};
  propset result, 0, p0;
  ret result;
}
```

Which represents that it creates a temporary variable named `result` being the currency zeroed. `propset` stores the value of `p0` on the first property(field) of `result`

#### Components

Since components in Slynx are a bit more complex than a simple struct, as they can execute some sort of code during runtime, they can be declared as contexts as well. This said, the implementation of the reactivity may be specific from one compiler to another
even though slynx in its PoC is idealized(might change since its not complete yet) to be OOP like.
A component like the following:

```slynx
func f(n: int): int {
  n * 2
}

component Counter {
  pub prop count = 0;
  Text {
    text: count
  }
  Text {
    text: f(count)
  }
}

func main():Component {
  Counter {}
  Counter {count: 12}
}
```

Can be compiled to an IR that looks like the following:

```slynxir
i32 f(i32) {
$entry:
  result = muli32 p0, 2;
  ret result;
}

component %Counter(i32) {
  %count: i32 = p0;
  
  #t0: specialized Text;
  #t1: specialized Text;
  @bind %count -> field #t0, 0;
  @bind %count |> f -> field #t1, 0;
}

AnyComponent main() {
$entry:
  Counter0 = %Counter(0); //0 = default value
  Counter1 = %Counter(12);
  ret Counter1
}
```

The idea is that instead of the value being optional on the IR as it's on the Slynx code, is to when the value is omitted, we instead of passing null, pass the explicitly the default value.
For some button that updates the state, suppose the following code:

```slynx

func f(n: int): int {
  n * 2
}

component Counter {
  pub prop count = 0;
  Text {
    text: count
  }
  Text {
    text: f(count)
  }
  Button {
    on_click: _ -> count += 1
  }
}

func main():Component {
  Counter {}
  Counter {count: 12}
}

```
which will generate:

```slynxir
special component %Text(%StrHandle) {
  %text: %StrHandle = p0;
}

i32 f(i32) {
$entry:
  result = muli32 p0, 2;
  ret result;
}

void Counter_count_update(%Counter) {
$entry:
  count = prop_get p0, 0;
  inc = addi32 count, 1;
  prop_set p0, 0, inc;
  @emit p0, %count;
  @rerender p0;
  ret
}

component %Counter(i32) {
  %count: i32 = p0;

  #t0: specialized Text;
  #t1: specialized Text;
  #b1: specialized Button;

  #b1.click -> Counter_count_update(self)

  @bind %count -> field #t0, 0;
  @bind %count |> f -> field #t1, 0;
  
}

AnyComponent main() {
$entry:
  Counter0 = %Counter(0); //0 = default value
  Counter1 = %Counter(12);
  ret Counter1;
}

```

Differently of default values, special values are primitives that are expected to exist on the runtime we are compiling to.

### UI Operations
Anything on the IR that initializes with '@' and is being used as an instruction, is an specific UI Operation, which determine what the UI itself should do. If being used as a value, then it's the visual reference to a handle of some internal string
On Components, @binds are way to determine which value on the component should update which dependency. On the %Counter example above, we had
```
@bind %count -> field #t0, 0;
@bind %count |> f -> field #t1, 0;
```

which means that, on %count update, it updates with the new value, the value of the field 0 of #t0. For the field 0 of #t1, it updates it using `%count |> f`, which means that the value of `call f, %count`, is used as the new value.
The `@emit p0, %count` on the function, tells that `p0` should execute its `%count` binds. And after executing them, send a re-render with @rerender.

* @bind: which follows `@bind %property |> func -> value`, means that on update of `%property` inside the component we are defining, updates the provided `value`
* @emit: which follows `@emit Component, %property`, means that it should execute the binds related to `%property` of the provided `Component`
* @rerender: which follows: `@rerender Component`, means that the `Component` should be re-rendered

### Instructions

#### Termination Operations

* br: Unconditional branch. Emits that the provided `label` should be executed.
* cbr: Conditional branch. Follows that `cbr value, $if_label, $else_label`. If the `value` is true, then it executes the `$if_label`, otherwhise `$else_labels`. Obviously, `value` must be a boolean
* ret: Only used inside functions, returns the provided value

#### Integer Operations

For now, arithmetic instructions are only defined for integer types.
Each instruction takes two operands of the same type and returns a value of that same type.
Unless explicitly stated otherwise, arithmetic instructions in this section are saturating.

Saturating semantics:

* If the exact result is representable in the operand type, that result is returned.
* If the exact result is above the type maximum, the type maximum is returned.
* If the exact result is below the type minimum, the type minimum is returned.
* For division, divide-by-zero traps. Signed `MIN / -1` saturates to `MAX`.

Addition:

* addi8: saturating add of two `i8` values, returns an `i8`
* addi16: saturating add of two `i16` values, returns an `i16`
* addi32: saturating add of two `i32` values, returns an `i32`
* addi64: saturating add of two `i64` values, returns an `i64`
* addi128: saturating add of two `i128` values, returns an `i128`
* addu8: saturating add of two `u8` values, returns a `u8`
* addu16: saturating add of two `u16` values, returns a `u16`
* addu32: saturating add of two `u32` values, returns a `u32`
* addu64: saturating add of two `u64` values, returns a `u64`
* addu128: saturating add of two `u128` values, returns a `u128`

Subtraction:

* subi8: saturating subtraction, returns an `i8`
* subi16: saturating subtraction, returns an `i16`
* subi32: saturating subtraction, returns an `i32`
* subi64: saturating subtraction, returns an `i64`
* subi128: saturating subtraction, returns an `i128`
* subu8: saturating subtraction, returns a `u8`
* subu16: saturating subtraction, returns a `u16`
* subu32: saturating subtraction, returns a `u32`
* subu64: saturating subtraction, returns a `u64`
* subu128: saturating subtraction, returns a `u128`

Multiplication:

* muli8: saturating multiplication, returns an `i8`
* muli16: saturating multiplication, returns an `i16`
* muli32: saturating multiplication, returns an `i32`
* muli64: saturating multiplication, returns an `i64`
* muli128: saturating multiplication, returns an `i128`
* mulu8: saturating multiplication, returns a `u8`
* mulu16: saturating multiplication, returns a `u16`
* mulu32: saturating multiplication, returns a `u32`
* mulu64: saturating multiplication, returns a `u64`
* mulu128: saturating multiplication, returns a `u128`

Division:

* divi8: saturating division, returns an `i8`
* divi16: saturating division, returns an `i16`
* divi32: saturating division, returns an `i32`
* divi64: saturating division, returns an `i64`
* divi128: saturating division, returns an `i128`
* divu8: saturating division, returns a `u8`
* divu16: saturating division, returns a `u16`
* divu32: saturating division, returns a `u32`
* divu64: saturating division, returns a `u64`
* divu128: saturating division, returns a `u128`

Wrapping variants:

Wrapping instructions are explicit and use modulo `2^N` arithmetic for the operand bit width.

Wrapping addition:

* wrapping_addi8
* wrapping_addi16
* wrapping_addi32
* wrapping_addi64
* wrapping_addi128
* wrapping_addu8
* wrapping_addu16
* wrapping_addu32
* wrapping_addu64
* wrapping_addu128

Wrapping subtraction:

* wrapping_subi8
* wrapping_subi16
* wrapping_subi32
* wrapping_subi64
* wrapping_subi128
* wrapping_subu8
* wrapping_subu16
* wrapping_subu32
* wrapping_subu64
* wrapping_subu128

Wrapping multiplication:

* wrapping_muli8
* wrapping_muli16
* wrapping_muli32
* wrapping_muli64
* wrapping_muli128
* wrapping_mulu8
* wrapping_mulu16
* wrapping_mulu32
* wrapping_mulu64
* wrapping_mulu128

#### Floating Point Operations

Floating point instructions follow IEEE-754 semantics and are not saturating.
Backends must preserve NaN, infinity and signed zero behavior.
Unless a backend cannot represent a specific edge case, the default rounding mode is round-to-nearest, ties-to-even.
Floating point divide-by-zero does not trap and follows IEEE-754 results.

Addition:

* addf32: adds two `f32` values and returns an `f32`
* addf64: adds two `f64` values and returns an `f64`

Subtraction:

* subf32: subtracts the second `f32` from the first and returns an `f32`
* subf64: subtracts the second `f64` from the first and returns an `f64`

Multiplication:

* mulf32: multiplies two `f32` values and returns an `f32`
* mulf64: multiplies two `f64` values and returns an `f64`

Division:

* divf32: divides the first `f32` by the second and returns an `f32`
* divf64: divides the first `f64` by the second and returns an `f64`

#### Logic Operations

* cmp, compares the first value to the second one, and returns `true` if they're equal, `false` if they're not
* cmpgt, compares the first value to the second one, and returns `true if the first is greater than the second one, and `false` otherwhise
* cmpgte, compares the first value to the second one, and returns `true` if the first is greater or equal to the second one, and `false` otherwhise
* cmplt, compares the first value to the second one, and returns `true` if the first is less than the second one, and `false` otherwise
* cmplte, compares the first value to the second one, and returns `true` if the first is less than or equal to the second one, and `false` otherwise
* cmpne, compares the first value to the second one, and returns `true` if they're not equal, and `false` otherwise

#### Strings Operations

* strconcat: Concats the first to the second string and returns a new copy.
* strcmp: Compares the first string to the second one, and returns 1u8 if they're equal, 0u8 if they're not.
* strcmpne: Compares the first string to the second one, and returns 1u8 if they're not equal, 0u8 otherwise.
* strlen: Returns the length of provided the string
