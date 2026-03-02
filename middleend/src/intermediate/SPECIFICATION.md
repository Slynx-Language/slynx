# Slynx IR
<<<<<<< HEAD
This IR is is intended to be used by underlying compilers, the language compiles down to JS only as a prove of concept, to show that it's possible to use this IR and compile to any target. Note that for so, this IR
tells what the backend compiling it should do instead of how, even though the language is a bit opinionated about the 'how'(mainly due to DOD).
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
=======

Slynx IR specifies what the backend compiling it should do instead of how, even though the language is a bit opiniously about how to do so.
First of all the IR follows SSA and is extremely typed.
The IR has got the concept of 'contexts' that are anything able to be runned, this means a struct isn't a context, because it by itself cannot execute code, but a method or a function are, as well as components that can have code
to be run, such as the reactivity their reactivity model.

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

```
>>>>>>> 61bf353 (chore: wrote basic specification of IR)
object S {
  property_1: int,
  property_2: float
}
```

<<<<<<< HEAD
<<<<<<< HEAD
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
  inc = add i32 p0, 1;
  br $end
$end:
  twice = mul i32, inc, 2;
  ret twice;
}
```

Since a label is a named block, it needs to terminate with a termination operation. Labels can receive parameters and can be entered through `br` and `cbr`, which pass values explicitly to the destination label(s). This replaces implicit branch-return style flows and keeps dataflow explicit for compilation and optimization.

Label parameters follow the `lpN` naming pattern, where `N` is the parameter index (starting at `0`), similarly to function parameters (`pN`):
```slynxir
$lbl(i32, u32):
  lb0_u = cast u32, lp0;
  r = addu32, lb0_u, lp1;
  ret r;
```
Branches pass label parameters by position:
```slynxir
$entry:
  n = add i32 p0, 1;
  br $lbl(n, 2u32);
```

 #### Functions
Functions are defined on the IR level as the following:
```slynxir
i32 #add(i32, i32) {
$entry:
  result = add i32 p0, p1;
  ret result;
}
```
 Where p0 is the first parameter and p1 the second one. The order of the names then is pN where N is the (N+1)th parameter of the function.
=======
Tuples use the same method, so a tuple in slynx denotated by
=======
Tuples(WIP) use the same method, so a tuple in slynx denotated by
>>>>>>> 4d4941f (chore: correct some informations about structs on IR)
```slynx

object T(int,float);

```

is defined by the same way as `S`, the only thing is how their fields are accessed on the code. (Tuples must be implemented in slynx yet)

#### Functions
Functions are defined on the IR level as the following:
```
int #add(int, int) {
  result = addi32 p0, p1;
  ret result;
}
```
Where p0 is the first parameter and p1 the second one. The order of the names then is pN where N is the (N+1)th parameter of the function.
>>>>>>> 61bf353 (chore: wrote basic specification of IR)
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

<<<<<<< HEAD
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
=======
```

struct %Currency {int}

%Currency currency_of(int) {
  result = %Currency{0};
  propset result, 0, p0;
  ret result;
}

```

Which represents that it creates a temporary variable named `result` being the currency zeroed. storefield stores the value of `p0` on the first field of `result`
>>>>>>> 61bf353 (chore: wrote basic specification of IR)

#### Strings
Strings on the IR are represented as internalized values. On the IR they live on a separated struct called Internalizer, and their access can be made via slices. Note that this IR expects them to be UTF8, and be represented as, inside the IR

```
struct %Handle {usize, usize}
```
which in slynx would be

```slynx
struct Handle {
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
```
struct %StrHandle {usize, usize}
@str0 = "jorge"; //this is a handle to the string "jorge", but its just for readability, because internally its the %StrHandle

struct %Person {%StrHandle, i32}

%Person main(void) {
  p = %Person{@str0, 44};
  ret p;
}
```

and so the compiler can determine how it should be done internally.<br>
The instructions for strings are:



#### Components

Since components in Slynx are a bit more complex than a simple struct, as they can execute some sort of code during runtime, they can be declared as contexts as well. This said, the implementation of the reactivity may be specific from one compiler to another
even though slynx in its PoC is idealized(might change since its not complete yet) to be OOP like.
A component like the following:

```slynx
<<<<<<< HEAD
=======

>>>>>>> 61bf353 (chore: wrote basic specification of IR)
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

<<<<<<< HEAD
```slynxir
i32 f(i32) {
$entry:
=======
```

int f(int) {
>>>>>>> 61bf353 (chore: wrote basic specification of IR)
  result = muli32 p0, 2;
  ret result;
}

<<<<<<< HEAD
component %Counter(i32) {
  %count: i32 = p0;
  
=======
component %Counter(int) {
  %count: int = p0;
>>>>>>> 61bf353 (chore: wrote basic specification of IR)
  #t0: specialized Text;
  #t1: specialized Text;
  @bind %count -> field #t0, 0;
  @bind %count |> f -> field #t1, 0;
}

AnyComponent main() {
<<<<<<< HEAD
$entry:
=======
>>>>>>> 61bf353 (chore: wrote basic specification of IR)
  Counter0 = %Counter(0); //0 = default value
  Counter1 = %Counter(12);
  ret Counter1
}
<<<<<<< HEAD
```

The idea is that instead of the value being optional on the IR as it's on the Slynx code, is to when the value is omitted, we instead of passing null, pass the explicitly the default value.
=======

```
The idea is that instead of the value being optional on the IR as it's on the Slynx code, is to when the value is ommited, we instead of passing null, pass the explictly the default value.
>>>>>>> 61bf353 (chore: wrote basic specification of IR)
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

<<<<<<< HEAD
```slynxir
special component %Text(%StrHandle) {
  %text: %StrHandle = p0;
}

i32 f(i32) {
$entry:
=======
```

special component %Text(bytes) {
  %text: bytes = p0;
}

int f(int) {
>>>>>>> 61bf353 (chore: wrote basic specification of IR)
  result = muli32 p0, 2;
  ret result;
}

void Counter_count_update(%Counter) {
<<<<<<< HEAD
$entry:
  count = prop_get p0, 0;
  inc = add i32 count, 1;
  prop_set p0, 0, inc;
  @emit p0, %count;
  @rerender p 0;
  ret
}

component %Counter(i32) {
  %count: i32 = p0;
=======
  count = prop_get p0, 0;
  inc = addi32 count, 1;
  prop_set p0, 0, inc;
  @emit %count;
}

component %Counter(int) {
  %count: int = p0;
>>>>>>> 61bf353 (chore: wrote basic specification of IR)

  #t0: specialized Text;
  #t1: specialized Text;
  #b1: specialized Button;

  #b1.click -> Counter_count_update(self)

  @bind %count -> field #t0, 0;
  @bind %count |> f -> field #t1, 0;
  
}

AnyComponent main() {
<<<<<<< HEAD
$entry:
  Counter0 = %Counter(0); //0 = default value
  Counter1 = %Counter(12);
  ret Counter1;
=======
  Counter0 = %Counter(0); //0 = default value
  Counter1 = %Counter(12);
>>>>>>> 61bf353 (chore: wrote basic specification of IR)
}

```

Differently of default values, special values are primitives that are expected to exist on the runtime we are compiling to.
<<<<<<< HEAD
<<<<<<< HEAD

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
* @hide: which follows `@hide Component, #child`, means that the `#child` of the provided `Component` should quit the UI tree. This instruction does not determine if the backend should kill or not the component, this is a compiler's choice
* @reveal: which follows `@reveal Component, #child`, does the opposite of `@hide`. It tells the `child` of the `Component` should be revealed on the UI. This instruction does not determine if the backend should create a new component or use a cached one, this is a compiler's choice


### Instructions

#### Termination Operations

* br: Unconditional branch. Follows `br $label(arg0, arg1, ...)` and transfers control to `$label`, binding arguments to its `lpN` parameters.
* cbr: Conditional branch. Follows `cbr value, $if_label(args...), $else_label(args...)`. If `value` is true, then it executes `$if_label`, otherwise `$else_label`. Obviously, `value` must be a boolean.
* ret: Only used inside functions, returns the provided value

#### General Operations
By default on the IR everything is moved, due to DOD, so to consume something that is not intended to be moved, we copy it via `copy` instruction.

* copy: copies the provided `value`. It follows as `copy value`.
* call: which follows: `call f, arg1, arg2, ...`, calls the provided function `f` passing `arg1`, `arg2`, ..., as parameters to it. A call is an expression
* cast: which follows: `cast ty, value` casts the provided `value`, copies the value and casts it to the provided `ty`
* select: which follows `select cond, v1, v2`, selects `v1` if `cond` is `true` and `v2` otherwise

#### Numeric Operations

Each instruction takes a type argument, and two operands of the same type and returns a value of that same type. Since this only covers integers, the `type` argument(also refered as `ty`) can be any primitive integer type
Unless explicitly stated otherwise, arithmetic instructions in this section are wrapping.

Addition:

* add: which follows `add ty, a,b` adds the provided `a` and `b` value asserting their type is the same as `ty` and returns the wrapping result.
* sub: which follows `sub ty, a,b` subtracts the provided `a` and `b` value asserting their type is the same as `ty` and returns the wrapping result.
* mul: which follows `mul ty, a,b` multiplies the provided `a` and `b` value asserting their type is the same as `ty` and returns the wrapping result.
* div: which follows `div ty, a,b` divides the provided `a` and `b` value asserting their type is the same as `ty` and returns the wrapping result.
* rem: which follows `rem ty, a,b` takes remainder value from the provided `a` and `b` value asserting their type is the same as `ty` and returns the wrapping result.


Saturing variants:

Saturing instructions are explicit and if the result would overflow, it instead, bounds the value to `MIN/MAX`.

Saturing addition:

* sat_add
* sat_sub
* sat_mul

#### Bit Operations(Only for integers)
* and: which follows `band ty, a,b`, stands for Bit AND, executes the AND operation from the provided `a` and `b` value asserting their type is the same as `ty` and returns the saturing result.
* or: which follows `bor ty, a,b`, stands for Bit OR, executes the OR operation from the provided `a` and `b` value asserting their type is the same as `ty` and returns the saturing result.
* xor: which follows `bxor ty, a,b`, stands for Bit XOR, executes the XOR operation from the provided `a` and `b` value asserting their type is the same as `ty` and returns the saturing result.
* not: which follows `bnot value`, standds for Bit Not, executes the NOT operation on the provided `value` and returns a copy of it. The type of this operation is inferred by the type of `value`.
* shl: which follows `shl ty, a,b` shifts to the left the value of `a` by `b` bits. Asserts their type is the same as `ty` and returns the saturing result.
* shr: which follows `shr ty, a,b` shifts to the right the value of `a` by `b` bits. Asserts their type is the same as `ty` and returns the saturing result. This is the logical implementation. So if `ty` is negative(thus, bit 1 to tell so), it will not keep
* ashr: which follows `ashr ty, a,b` shifts to the right the value of `a` by `b` bits. Asserts their type is the same as `ty` and returns the saturing result. This is the arithmetical implementation, so the negative bit keeps. This is the same as N / 2, for N of any int type

#### Logic Operations

* cmp, compares the first value to the second one, and returns `true` if they're equal, `false` if they're not
* cmpgt, compares the first value to the second one, and returns `true` if the first is greater than the second one, and `false` otherwhise
* cmpgte, compares the first value to the second one, and returns `true` if the first is greater or equal to the second one, and `false` otherwhise
* cmplt, compares the first value to the second one, and returns `true` if the first is less than the second one, and `false` otherwise
* cmplte, compares the first value to the second one, and returns `true` if the first is less than or equal to the second one, and `false` otherwise
* cmpne, compares the first value to the second one, and returns `true` if they're not equal, and `false` otherwise

#### Idealized For The Future
These operations are idealized to be implemented on the future and for the V1 are not being implemented. Note that since these are only IDEALIZED, they might and probably WILL change
=======
>>>>>>> 61bf353 (chore: wrote basic specification of IR)
=======

#### Instructions

##### Integer Operations

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

##### Floating Point Operations

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

##### Logic Operations

* cmp, compares the first value to the second one, and returns 1u8 if they're equal, 0u8 if they're not
* cmpgt, compares the first value to the second one, and returns 1u8 if the first is greater than the second one, and 0u8 otherwhise
* cmpgte, compares the first value to the second one, and returns 1u8 if the first is greater or equal to the second one, and 0u8 otherwhise
* cmplt, compares the first value to the second one, and returns 1u8 if the first is less than the second one, and 0u8 otherwise
* cmplte, compares the first value to the second one, and returns 1u8 if the first is less than or equal to the second one, and 0u8 otherwise
* cmpne, compares the first value to the second one, and returns 1u8 if they're not equal, and 0u8 otherwise
<<<<<<< HEAD
>>>>>>> ccdd92f (chore: wrote about more instructions)
=======

##### Strings Operations

* strconcat: Concats the first to the second string and returns a new copy.
* strlen: Returns the length of the string
>>>>>>> 32a7322 (chore: corrected wrong instruction names)
