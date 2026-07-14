# Arrays, Vectors and Slices

In the language there are going to be 3 types of basic collections: 
Arrays which are represented by `[N]T` where N is a number, such as `[64]int`, `[128]bool`, etc. Vectors, which are represented by `[]T`, WITHOUT a numeric, which explicits that the size is not known, so, dynamic, so a vector.

And lastly a slice which is T[:].

## Creation
For creating an array and a vector, the syntax is the same: `let a = [1uint8,2,3,4];` which by default will be an [N]T, in this case, `[4]uint8`. To represent it such as a vector it must be determined such as `let a: []uint8 = [1,2,3,4]`.
Expressions prefixed by `[N]` understands that the next value is an array with N size, and the value of all the positions is the given value, such as `[44]0` is the same as [0,0,0,0,..44 0]. Same rule applies to vectors.

## Slices
Slices are operations that simply take a view into an array/vector. Slices then can be understood as references and local values. Due to so, the rules and the nature of the language, slices cannot be saved into structs(see memory-model.md)
and can only be used within temporary things such as functions and parameters.

To create a slice the idea is pretty simple: expression[range], where range is `N:M`, which slices from N to M `N:` which slices from N to the end, `:M` which slices start from the end, `:` which contains everything. The type then will be `[:]T`, which is a slice of T. A slice can be sliced as well.
