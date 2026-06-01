# String Interning

A way to make it easier to check for strings is by interning them. The interning means that we get a string such as 'maria e jose' and store it somewhere, back, we got its ID. The idea is to make its ID small and fast to copy.

This is made mainly because, if we got a bunch of checks for the names instead of checking if S == 'maria e jose', which checks about 12 bytes(according to utf8 which is what rust relies on), we simply check for the ID. This is much better because of mainly 2 reasons: 
  1. The size is always fixed(in the case of this language, 8 bytes per ID), and so it is easier for the compiler to compare, since it's a u64 internally.
  2. Prevents heap allocation, so no mallocs and heap indirections are required.

The internalizer struct of this language can be found at crates/common/src/symbols.rs.

## How it works 
Internally the ID of a string is just an encoded pointer to it. You can see so by checking that SymbolPointer is just a wrapper over u64.

On the implementation, we can se a bit of magic such as `ptr << 16 | length`. What happens is that the string we are interning gets pushed to the internal string pool, and so on interning 'maria', appends 'maria' into the string pool and on interning 'jose' the same, which makes the string pool become something like 'mariajose' due to sequential appends of the strings. The main idea then is that the first 16 bits of it are the length of the string and the 48 higher bits are where it is located at. The flow of it might be such as like the following:

StringPool = []
intern("maria")
StringPool = ["m","a","r","i","a"]
MariaPtr = 5

The reason is that 'maria' starts at the 0 index, so, the 48 higher bits are '0', and the length of 'maria' is 5, so the 16 lower bits are '5'. So the result is:
Higher 48 Bits: 0
Lower 16 Bits: 5

StringPool = ["m","a","r","i","a"]
intern("jose")
StringPool = ["m","a","r","i","a", "j","o", "s", "e"]
JosePtr = 5 << 16 | 4, or 327684

The reason why 'jose' id is 327684, is because 5 << 16 is 327680, which represent that, the 48 higher bits of the number are '000..101', and the '4' comes from the '| length' where the length of 'jose' is 4, so 0 | 4 = 4.

### Limits
This limits the internal string pool to have about 281 trillion of bytes, so.. yeah, i dont think a project will use that much of strings. 
But a more possible limitation is that due to so, strings must be at max 65kb. The main idea for so its because I really dont think anyone will write raw 65kb of content on a slynx file.

## Why a Phantom?

The phantom on the SymbolPointer is because both IR and HIR need to make interning but they cannot use the other's symbols pointer. The reason for so its because the IR must be agnostic from the HIR, so it needs to remake the interning. 

Then the phantom is used to tell who one is creating it at the moment and not mix with the one's of another
