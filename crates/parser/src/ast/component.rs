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
    ///Represents the definition of a property inside a component. Such as component C {prop value=5;}
    Property {
        name: SymbolPointer,
        modifier: VisibilityModifier,
        ty: Option<Spanned<DedupPoolId<Type>>>,
        rhs: Option<Spanned<DedupPoolId<ASTExpression>>>,
    },
    ///Represents the definition of a component that contains a name. The name is merely to make it able to reference contents of the child properly.
    ///```slx
    /// component C {
    ///     child=MyChild {}
    /// }
    /// ```
    NamedChild {
        name: SymbolPointer,
        child: Spanned<ComponentExpression>,
    },
    Child(Spanned<ComponentExpression>),
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ComponentMemberValue {
    Assign {
        prop_name: SymbolPointer,
        rhs: Spanned<DedupPoolId<ASTExpression>>,
    },
    NamedChild {
        name: SymbolPointer,
        child: ComponentExpression,
    },
    Child(ComponentExpression),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ComponentExpression {
    pub name: Spanned<DedupPoolId<Type>>,
    pub values: Vec<ComponentMemberValue>,
}
