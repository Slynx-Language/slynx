mod declarations;
mod expressions;
mod types;
mod views;
use common::VisibilityModifier;
pub use views::*;
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Visible<T> {
    pub visibility: VisibilityModifier,
    pub data: T,
}

impl<T> Visible<T> {
    pub fn new(visibility: VisibilityModifier, data: T) -> Self {
        Self { visibility, data }
    }
    pub fn is_visible(&self) -> bool {
        self.visibility == VisibilityModifier::Public
    }
}
