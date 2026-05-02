mod component;
mod expression;
mod types;

pub use component::*;
pub use expression::*;
pub use types::*;

#[derive(Default, Debug, Clone, Copy)]
pub enum VisibilityModifier {
    ///Property visible to everyone
    Public,
    ///Property visible only by the one defining it.
    #[default]
    Private,
    ///Property visible only for the children. Only usable on Components.
    ChildrenPublic,
    ///Property visible only for the parents. Only usable on Components.
    ParentPublic,
}
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

#[derive(Debug)]
///Some statement on the code, a statement not necessarily have value, in general expressions do.
pub struct ASTStatement {
    pub kind: ASTStatementKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ASTStatementKind {
    Var {
        name: String,
        ty: Option<GenericIdentifier>,
        rhs: ASTExpression,
    },
    MutableVar {
        name: String,
        ty: Option<GenericIdentifier>,
        rhs: ASTExpression,
    },
    Assign {
        ///The Left hand side of the assign, or, the one that will receive the value of `rhs`
        lhs: ASTExpression,
        rhs: ASTExpression,
    },

    While {
        condition: ASTExpression,
        body: Vec<ASTStatement>,
    },

    Expression(ASTExpression),
}

#[derive(Debug)]
pub struct ASTDeclaration {
    pub kind: ASTDeclarationKind,
    pub span: Span,
}

#[derive(Debug)]
pub struct ObjectField {
    pub visibility: VisibilityModifier,
    pub name: TypedName,
}

#[derive(Debug)]
pub enum ASTDeclarationKind {
    Alias {
        name: GenericIdentifier,
        target: GenericIdentifier,
    },
    ObjectDeclaration {
        name: GenericIdentifier,
        fields: Vec<ObjectField>,
    },
    ComponentDeclaration {
        name: GenericIdentifier,
        members: Vec<ComponentMember>,
    },
    FuncDeclaration {
        name: GenericIdentifier,
        args: Vec<TypedName>,
        return_type: GenericIdentifier,
        body: Vec<ASTStatement>,
    },
}

impl ASTExpression {
    pub fn is_assignable(&self) -> bool {
        matches!(
            self.kind,
            ASTExpressionKind::Identifier(_) | ASTExpressionKind::FieldAccess { .. },
        )
    }
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
