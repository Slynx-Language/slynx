use std::ops::Index;

#[derive(Debug, Default)]
///A Pool of linear strings. This is used to don't use a lot of Strings inside a vector, since strings are Vec<u8> internally
///having a Vec<String> would be pretty unecessary when talking about lengths
pub struct StringPool {
    pool: Vec<u8>,
    len: usize,
}

#[derive(Debug, Clone)]
///A handle that is used to access some String inside a String pool.
pub struct StringHandle {
    index: usize,
    len: usize,
    ///The index of the last inserted byte
    last_index: usize,
}

impl StringPool {
    ///Initializes a new pool without strings
    pub fn new() -> Self {
        Self {
            pool: Vec::new(),
            len: 0,
        }
    }

    ///Returns the amount of strings allocated on this pool
    pub fn len(&self) -> usize {
        self.len
    }

    ///Returns the amount of strings allocated on this pool
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    ///Retrieves how many bytes were allocated
    pub fn bytesize(&self) -> usize {
        self.pool.len()
    }
    ///Inserts the provided `string` in this pool and returns it's handle to be accessed.
    ///The string is immutable
    pub fn insert_string(&mut self, str: &str) -> StringHandle {
        let bytes = str.as_bytes();
        let handle = StringHandle {
            index: self.bytesize(),
            len: bytes.len(),
            last_index: self.bytesize() + bytes.len() - 1,
        };
        self.pool.extend_from_slice(bytes);
        self.len += 1;
        handle
    }
}

impl Index<StringHandle> for StringPool {
    type Output = str;
    fn index(&self, index: StringHandle) -> &Self::Output {
        unsafe { str::from_utf8_unchecked(&self.pool[index.index..index.len + index.index]) }
    }
}
impl Index<&StringHandle> for StringPool {
    type Output = str;
    fn index(&self, index: &StringHandle) -> &Self::Output {
        unsafe { str::from_utf8_unchecked(&self.pool[index.index..index.len + index.index]) }
    }
}
impl StringHandle {
    ///Slices this handle returning a new one with the provided `range`
    pub fn slice<R: std::ops::RangeBounds<usize>>(&self, range: R) -> StringHandle {
        use std::ops::Bound::*;

        let start = match range.start_bound() {
            Included(&i) => i,
            Excluded(&i) => i + 1,
            Unbounded => 0,
        };

        let end = match range.end_bound() {
            Included(&i) => i + 1,
            Excluded(&i) => i,
            Unbounded => self.len,
        };

        assert!(start <= end && end <= self.len && self.len <= self.last_index);

        StringHandle {
            index: self.index + start,
            len: end - start,
            last_index: self.last_index,
        }
    }
}
