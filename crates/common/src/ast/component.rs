use crate::ast::{ASTExpression, GenericIdentifier, Span, VisibilityModifier};
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
        rhs: Option<ASTExpression>,
    },
    Child(ComponentExpression),
}
#[derive(Debug)]
pub enum ComponentMemberValue {
    Assign {
        prop_name: String,
        rhs: ASTExpression,
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
