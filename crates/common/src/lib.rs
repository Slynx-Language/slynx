pub mod symbols;
pub use symbols::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
///The representation of the bounds of something on the code.
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    ///Merges this span with the given `target`. The returned span will have the initial position of this one, and the final position of the given `target`
    pub fn merge_with(mut self, target: Self) -> Self {
        self.end = target.end;
        self
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
///Some operator on the code. Something like, +, - , *, /, &, &&, etc
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
