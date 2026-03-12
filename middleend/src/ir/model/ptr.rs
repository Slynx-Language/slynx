use std::marker::PhantomData;

///A Pointer to something on the IR. This is a logical pointer composed by 48 bits(higher bits) that determine where the thing we are pointing to is located on the IR, and a length of 16bits to know how much of it we have as well.
///This of this as a slice, but instead of containing data on the actual memory, it takes on the contents of the IR
#[derive(Debug, Copy, Clone)]
pub struct IRPointer<T> {
    inner: u64,
    data: PhantomData<T>,
}

impl<T> IRPointer<T> {
    ///Creates a new IRPointer with the provided `ptr` getting the next `len` values after it.
    pub fn new(ptr: usize, len: usize) -> Self {
        Self {
            inner: (ptr << 16 | (len & 0xffff)) as u64,
            data: PhantomData,
        }
    }

    #[inline]
    ///Gets the raw value of the pointer
    pub unsafe fn raw(&self) -> u64 {
        self.inner
    }
    
    #[inline]
    ///Gets the pointer part of the IRPointer, i.e. the location of the thing we are pointing to on the IR.
    pub fn ptr(&self) -> usize {
        (self.inner >> 16) as usize
    }
    
    #[inline]
    ///Gets the length part of the IRPointer, i.e. how many values we have after the pointer.
    pub fn len(&self) -> usize {
        self.inner as usize & 0xffff
    }
}
