mod component;
mod declarations;
mod expression;
mod imports;
mod statement;
mod types;

use std::hash::Hash;

use common::Span;
pub use common::VisibilityModifier;
pub use component::*;
pub use declarations::*;
pub use expression::*;
pub use imports::*;
pub use statement::*;
pub use types::*;

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
