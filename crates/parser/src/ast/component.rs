use crate::ast::{ASTExpression, GenericIdentifier, VisibilityModifier};
use common::{Span, pool::PoolId};
#[derive(Debug)]
///A member on a component, this can be a property or a child expression
pub struct ComponentMember {
    pub kind: ComponentMemberKind,
    pub span: Span,
}
#[derive(Debug)]
pub enum ComponentMemberKind {
    Property {
        name: String,
        modifier: VisibilityModifier,
        ty: Option<GenericIdentifier>,
        rhs: Option<PoolId<ASTExpression>>,
    },
    Child(ComponentExpression),
}
#[derive(Debug)]
pub enum ComponentMemberValue {
    Assign {
        prop_name: String,
        rhs: PoolId<ASTExpression>,
        span: Span,
    },
    Child(ComponentExpression),
}

#[derive(Debug)]
pub struct ComponentExpression {
    pub name: GenericIdentifier,
    pub values: Vec<ComponentMemberValue>,
    pub span: Span,
}
