use crate::{
    FuncDeclaration, SymbolPointer,
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
    ///Represents the definition of a property inside a component. Such as `component C {prop value=5;}`
    Property {
        name: SymbolPointer,
        modifier: VisibilityModifier,
        callbacks: Vec<FuncDeclaration>,
        ty: Option<Spanned<DedupPoolId<Type>>>,
        rhs: Option<Spanned<DedupPoolId<ASTExpression>>>,
    },
    ///Represents a component input. Such as `component C {in value: T}`. Does not require visibility because
    /// it is always visible for outside
    Input {
        name: SymbolPointer,
        ty: Spanned<DedupPoolId<Type>>, //since this one musn't have a default value, the type is required
    },
    ///Represents a component view. Such as `component C { view myview = property * 2;}`
    View {
        modifier: VisibilityModifier,
        name: SymbolPointer,
        rhs: Spanned<DedupPoolId<ASTExpression>>,
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
