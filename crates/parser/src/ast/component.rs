use crate::{
    SymbolPointer,
    ast::{ASTExpression, Type, VisibilityModifier},
};
use common::{Span, Spanned, pool::DedupPoolId};
#[derive(Debug)]
///A member on a component, this can be a property or a child expression
pub struct ComponentMember {
    pub kind: ComponentMemberKind,
    pub span: Span,
}
#[derive(Debug)]
pub enum ComponentMemberKind {
    Property {
        name: SymbolPointer,
        modifier: VisibilityModifier,
        ty: Option<Spanned<DedupPoolId<Type>>>,
        rhs: Option<Spanned<DedupPoolId<ASTExpression>>>,
    },
    Child(Spanned<ComponentExpression>),
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ComponentMemberValue {
    Assign {
        prop_name: SymbolPointer,
        rhs: Spanned<DedupPoolId<ASTExpression>>,
    },
    Child(ComponentExpression),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ComponentExpression {
    pub name: Spanned<DedupPoolId<Type>>,
    pub values: Vec<ComponentMemberValue>,
}
