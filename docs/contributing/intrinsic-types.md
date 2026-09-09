# Intrinsic Types

Intrinsic types are types that are built into the language, more specifically, an internal mapping from some name to a type. Not necessarily the name of the type is referenced to it. For example:
```slynx
@intrinsic("color")
pub object Oklch {}
```


Everytime we want to access the intrinsic type `color` we will access the type `Oklch`. This is mainly used for type checking and code completion. For example when writing some style, the type checker should not contain the
reference to the `Oklch` type by mapping it somehow, instead if tries to find the `color` intrinsic type. This is mainly good because it doesn't require the compiler to support every type that is intrinsic and should be known ahead of time. They can simply be defined in the userland(even though it isn't the userland yet). 

The unique downside is that the compiler must have a way to know what is intrinsic and what is not. Then the intrinsics must be imported on the code when loading, otherwise the compiler will not know who they are, and fail the compilation, but this is easily resolved via importing some module that contains the intrinsic types, or making auto imports of them on the main file, wihch might be the better option since the HIR generation is lazy, so if they aren't
used on the code, they are not generated
