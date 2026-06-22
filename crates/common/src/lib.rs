pub mod pool;
mod span;
pub use span::*;
pub mod symbols;
pub use symbols::*;

/// Visibility of a declaration.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisibilityModifier {
    /// Visible to everyone.
    Public,
    /// Visible only within the defining file.
    #[default]
    Private,
    /// Visible only to children (components only).
    ChildrenPublic,
    /// Visible only to parents (components only).
    ParentPublic,
}

///Some operator on the code. Something like, +, - , *, /, &, &&, etc
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Operator {
    Add,
    Sub,
    Star,
    Slash,
    Equals,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    LogicAnd,
    LogicOr,
    And,
    Or,
    RightShift,
    LeftShift,
    Xor,
}

impl Operator {
    pub fn is_logical(&self) -> bool {
        matches!(
            self,
            Self::LogicAnd
                | Self::LogicOr
                | Self::Equals
                | Self::GreaterThan
                | Self::GreaterThanOrEqual
                | Self::LessThan
                | Self::LessThanOrEqual
        )
    }
}
