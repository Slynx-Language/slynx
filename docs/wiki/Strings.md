# Strings
In slynx, strings are still very unuseful. The is nothing too complex about them, so most of this documentation will talk about
what is idealized for them on the long run, rather than what is actually implemented at the time.

## Definition
Strings are a primitive for the compiler and can be defined such as: `"some string"`. Their type on slynx is simply: `str`

## Goals

At the moment the strings are immutable and cannot do much yet. The goal then is to create abstractions over it.
Ideally string literals are intended to be constant values and instead of mutating them in place, it should be copied to a buffer that accepts mutation. So there will be a difference
between `str` and `MutableString`(this might be a good name). 

## Interpolation

Interpolating strings is idealized to contain the following syntax: `"my literal \(expression)"`. The idea is to anything inside `\()` should be evaluated as an expression, and the content be inserted in the middle.

## String encoding
The encoding of strings is intended to be utf8 and length based, even though the actual goal is to make the internal representation of strings target specific, so defined by the one compiling the code.

## Characters
At the moment no 'char' type exists on the codebase but it's idealized to be with `'c'` expressions. Nothing too complex, but needs to be mentioned

## Multi Line
Multi line strings are yet not implemented and no idealization related to them has been made. So this topic is open to contribution more than other topics
