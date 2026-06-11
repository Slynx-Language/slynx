# Attributes

On slynx attributes are such as decorators on typescript, but the metadata is totally compile time and may or may not give some permissions to what is being made. This is being idealized mainly for 
exposing easily functions that shouldn't do something. Such as a function that calculates something requiring an internal fs. So this capability system explicitly says what are the capabilities of one function, and so, what other functions need to be able
to call it as well. 
For example:

@capabilities(fs(write))
func f() {
  ...
}
every function that wants to call 'f' MUST have declare that they are 'fs(write)' as well.

By now this capability system is being idealized to create intrinsics on the compiler via @intrinsics('somename');
Later it'll be able to register comptime metadata to the declarations, but only during compile time
