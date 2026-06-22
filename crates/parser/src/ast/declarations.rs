use common::{Span, Spanned, VisibilityModifier, pool::PoolId};

use crate::{
    ASTExpression, ASTStatement, ComponentMember, GenericIdentifier, ObjectField,
    StyleSheetStatement, SymbolPointer, TypedName,
};

#[derive(Debug)]
///Represents a @name(...args). An Attribute is mainly used to define some metadata about the given declaration.
pub struct ASTAttribute {
    pub name: SymbolPointer,
    pub args: Vec<SymbolPointer>,
}

#[derive(Debug)]
pub struct ObjectMethod {
    pub method_name: Spanned<PoolId<GenericIdentifier>>,
    pub arguments: Vec<Spanned<TypedName>>,
    pub return_type: Spanned<PoolId<GenericIdentifier>>,
    pub body: Vec<Spanned<PoolId<ASTStatement>>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct AliasDeclaration {
    pub name: Spanned<PoolId<GenericIdentifier>>,
    pub target: Spanned<PoolId<GenericIdentifier>>,
    pub span: Span,
    pub visibility: VisibilityModifier,
}
#[derive(Debug)]
pub struct ObjectDeclaration {
    pub name: Spanned<PoolId<GenericIdentifier>>,
    pub fields: Vec<ObjectField>,
    pub methods: Vec<ObjectMethod>,
    pub attributes: Vec<ASTAttribute>,
    pub span: Span,
    pub visibility: VisibilityModifier,
    pub external: bool,
}
#[derive(Debug)]
pub struct ComponentDeclaration {
    pub name: Spanned<PoolId<GenericIdentifier>>,
    pub members: Vec<ComponentMember>,
    pub attributes: Vec<ASTAttribute>,
    pub visibility: VisibilityModifier,
    pub span: Span,
}
#[derive(Debug)]
pub struct FuncDeclaration {
    pub name: Spanned<PoolId<GenericIdentifier>>,
    pub args: Vec<Spanned<TypedName>>,
    pub return_type: Spanned<PoolId<GenericIdentifier>>,
    pub body: Vec<Spanned<PoolId<ASTStatement>>>,
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub visibility: VisibilityModifier,
    pub span: Span,
    pub external: bool,
}
#[derive(Debug)]
pub struct StyleSheet {
    pub name: Spanned<PoolId<GenericIdentifier>>,
    pub args: Vec<Spanned<TypedName>>,
    pub usages: Vec<Spanned<PoolId<ASTExpression>>>,
    pub body: Vec<Spanned<StyleSheetStatement>>,
    pub attributes: Vec<ASTAttribute>,
    pub visibility: VisibilityModifier,
    pub span: Span,
}
#[derive(Debug)]
pub struct StaticDeclaration {
    pub name: SymbolPointer,
    pub ty: Spanned<PoolId<GenericIdentifier>>,
    pub value: Option<Spanned<PoolId<ASTExpression>>>, //option because, if not provided, it yet can be used, even though might lead to runtime bugs. Should be None only on externs
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub visibility: VisibilityModifier,
    pub span: Span,
    pub external: bool,
}

#[derive(Debug)]
pub struct StyleState {
    pub states: Vec<SymbolPointer>,
    pub duration: Option<Spanned<PoolId<ASTExpression>>>,
    pub transition_curve: Option<SymbolPointer>,
}

impl StyleState {
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            duration: None,
            transition_curve: None,
        }
    }
}
