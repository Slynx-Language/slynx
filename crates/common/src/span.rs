use std::hash::Hash;

///The representation of the bounds of something on the code.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    ///Merges this span with the given `target`. The returned span will have the initial position of this one, and the final position of the given `target`
    pub fn merge_with(mut self, target: Self) -> Self {
        self.end = target.end;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub data: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(data: T, span: Span) -> Self {
        Spanned { data, span }
    }
}

impl<T> Hash for Spanned<T>
where
    T: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}
impl<T> PartialEq for Spanned<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}
impl<T> Eq for Spanned<T> where T: PartialEq {}
