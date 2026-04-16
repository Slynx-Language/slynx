use std::{marker::PhantomData, ops::Range};

///A Pointer to something on the IR. This is a logical pointer composed by 48 bits(higher bits) that determine where the thing we are pointing to is located on the IR, and a length of 16bits to know how much of it we have as well.
///Think of this as a slice, but instead of containing data on the actual memory, it takes on the contents of the IR
#[derive(Debug, Copy, Hash, PartialEq, Eq)]
pub struct IRPointer<T, const N: usize = 0> {
    inner: u64,
    data: PhantomData<T>,
}

impl<T, const N: usize> Clone for IRPointer<T, N> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            data: PhantomData,
        }
    }
}

impl<T, const N: usize> Default for IRPointer<T, N> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T, const N: usize> IRPointer<T, N> {
    ///Creates a new IRPointer with the provided `ptr` getting the next `len` values after it.
    pub fn new(ptr: usize, len: usize) -> Self {
        Self {
            inner: (ptr << 16 | (len & 0xffff)) as u64,
            data: PhantomData,
        }
    }

    ///Creates a new IRPointer with the same pointer but a different length.
    pub fn with_length<const M: usize>(mut self) -> IRPointer<T, M> {
        if M != 0 {
            self.set_length(M)
        };
        IRPointer {
            inner: self.inner,
            data: PhantomData,
        }
    }

    ///Returns a pointer that moves `n` values forward
    pub fn ptr_to(&self, n: usize) -> IRPointer<T, 1> {
        let mut out = self.clone();
        out.set_ptr(self.ptr() + n);
        out.with_length()
    }

    ///If the length of this Pointer is >1, then this retrieves a pointer to the last element
    pub fn ptr_to_last(&self) -> IRPointer<T, 1> {
        self.ptr_to(self.len() - 1)
    }

    pub fn with_runtime_length(mut self, len: usize) -> IRPointer<T, 0> {
        self.set_length(len);
        self.with_length()
    }

    #[inline]
    pub fn has_dynamic_length(&self) -> bool {
        N == 0
    }

    #[inline]
    ///Returns a null IRPointer
    pub fn null() -> Self {
        Self {
            inner: 0,
            data: PhantomData,
        }
    }

    #[inline]
    ///Gets the raw value of the pointer
    ///# Safety
    /// The value won't represent the length nor the pointer inside the IR
    pub unsafe fn raw(&self) -> u64 {
        self.inner
    }

    #[inline]
    ///Sets the pointer part of the IRPointer to the provided value.
    pub fn set_ptr(&mut self, ptr: usize) {
        self.inner = (ptr << 16 | (self.len() & 0xffff)) as u64;
    }

    #[inline]
    ///Gets the pointer part of the IRPointer, i.e. the location of the thing we are pointing to on the IR.
    pub fn ptr(&self) -> usize {
        (self.inner >> 16) as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        N == 0 || (self.inner & 0xffff) == 0
    }

    #[inline]
    ///Gets the length part of the IRPointer, i.e. how many values we have after the pointer.
    pub fn len(&self) -> usize {
        if N == 0 {
            self.inner as usize & 0xffff
        } else {
            N
        }
    }

    #[inline]
    ///Increases the length part of the IRPointer by 1.
    pub fn increase_length(&mut self) {
        let lhs = self.len() + 1;
        self.set_length(lhs);
    }
    #[inline]
    ///Sets the length part of the IRPointer to the provided value.
    pub fn set_length(&mut self, len: usize) {
        self.inner = (self.ptr() << 16 | (len & 0xffff)) as u64;
    }

    #[inline]
    ///Retrieves the ranges this pointer goes to. The range is the same as `ptr..ptr+len`
    pub fn range(&self) -> Range<usize> {
        self.ptr()..self.ptr() + self.len()
    }
}
