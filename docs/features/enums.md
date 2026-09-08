# Enums
Enums are a way to define a type that can have one of several possible values. Each value is called a variant.
They can have associated values, the idea then is that a variant contains a value that is tied to it. This one can be anything, even a generic type.

## Definition
An basic enum for optional types can be achieved via

```slynx
enum Option<T> {
    Some(T),
    None,
}
```
and its creation can be defined as:

```slynx
let opt: Option<int> = Option.Some(42);
```


## Matches Expression
Matches expressions allow you to check if the value of an enum matches a specific variant and its inner values. This is the entry point for pattern matching, even though it's not implemented yet. 
It can be used as follows:

```slynx
if opt matches Some(44) {
    print("opt is some and 44");
}
```
This is idealized to be extended on the future to allow pattern matching in general, but at the moment it is used only for enum variants.
The check can use only the name of the variant and its inner values, so if an enum is defined as
```slynx
enum User {
    NormalUser(User),
    AdminUser(User),
    Owner(Owner),
}
```
it can be matched as `user matches AdminUser(_)` to check if it is an admin user. It is pretty dumb at the moment and thus `_` is not working as expected, the main reasion is cause it checks it as an identifier and tries
to check if the inner value of the variant is the same as the one provided. Thus something like `num matches Some(mynum)` ideally checks if num is a `Some` variant and its inner value is equal to `mynum`.
# Enums

Enums are a way to define a type that can have one of several possible values. Each value is called a variant.

Variants can have associated values. An associated value is a value tied to a specific variant and can have any type, including a generic type.

## Definition

A basic enum for optional types can be defined as:

```slynx
enum Option<T> {
    Some(T),
    None,
}
```

The enum can then be instantiated by providing the value associated with the variant:

```slynx
let opt: Option<int> = Option.Some(42);
```

A variant does not need to have an associated value. For example, `None` above is a variant without any associated value, while `Some` contains a value of type `T`.

Associated values can use concrete types, generic types, or other types defined in the program.

## Matches Expression

The `matches` expression allows checking whether an enum value is a specific variant and whether its associated values match the values provided in the expression.

It is the entry point for pattern matching, although general pattern matching is not implemented yet.

For example:

```slynx
if opt matches Some(44) {
    print("opt is some and 44");
}
```

At the moment, `matches` only supports checking enum variants and their associated values. The syntax is intentionally designed to be extended into general pattern matching in the future.

The variant name and its associated values are specified directly in the expression. For example, given:

```slynx
enum User {
    NormalUser(User),
    AdminUser(User),
    Owner(Owner),
}
```

A value can be checked against the `AdminUser` variant with:

```slynx
user matches AdminUser(_)
```

However, `_` is not currently implemented as a wildcard. The current implementation treats it as an identifier and attempts to compare the associated value against it.

More generally, a check such as `num matches Some(mynum)` currently means that `num` must be a `Some` variant and that its associated value must be equal to `mynum`.

This behavior is intentionally simple for now. In the future, `matches` is intended to evolve into general pattern matching, where constructs such as `_`, bindings, nested patterns, and other pattern forms can be supported.
