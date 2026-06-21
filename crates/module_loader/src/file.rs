#[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq)]
///An ID to represent a file
pub struct FileId(u32);
impl FileId {
    pub fn from_raw(value: u32) -> Self {
        Self(value)
    }
    pub fn as_raw(&self) -> u32 {
        self.0
    }
    fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}
