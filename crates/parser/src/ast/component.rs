use crate::{
    SymbolPointer,
    ast::{ASTExpression, GenericIdentifier, VisibilityModifier},
};
use common::{Span, Spanned, pool::PoolId};
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
        ty: Option<Spanned<PoolId<GenericIdentifier>>>,
        rhs: Option<Spanned<PoolId<ASTExpression>>>,
    },
    Child(Spanned<ComponentExpression>),
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ComponentMemberValue {
    Assign {
        prop_name: SymbolPointer,
        rhs: PoolId<ASTExpression>,
    },
    Child(ComponentExpression),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ComponentExpression {
    pub name: PoolId<GenericIdentifier>,
    pub values: Vec<ComponentMemberValue>,
}
