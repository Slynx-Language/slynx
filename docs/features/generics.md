# Generics

On the language the concept of generics is implemented via monomorphization. The idea at the moment is pretty simple. For a struct T that defines generics K, for each ocurrance of T where K is different  the compiler generates an specific implementation.
For example:
```slynx
struct Vector<K> {
    ...
}
```
If we got Vector<int>, the compiler generates a specific implementation of Vector<int>. It literally copy pastes the code of Vector<int> and replaces the generic type K with int.
This generates code fully optimized, but the downside is that this generates slower because it copy pastes.
The generics at the moment do not have much features, but others are idealized, such as interfaces, trait-bounds, const generics, etc
