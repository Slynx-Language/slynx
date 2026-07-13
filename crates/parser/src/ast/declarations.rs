use common::{Span, Spanned, VisibilityModifier, pool::DedupPoolId};

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
    pub method_name: Spanned<DedupPoolId<GenericIdentifier>>,
    pub arguments: Vec<Spanned<TypedName>>,
    pub return_type: Spanned<DedupPoolId<GenericIdentifier>>,
    pub body: Vec<Spanned<DedupPoolId<ASTStatement>>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct AliasDeclaration {
    pub name: Spanned<DedupPoolId<GenericIdentifier>>,
    pub target: Spanned<DedupPoolId<GenericIdentifier>>,
    pub span: Span,
    pub visibility: VisibilityModifier,
}
#[derive(Debug)]
pub struct ObjectDeclaration {
    pub name: Spanned<DedupPoolId<GenericIdentifier>>,
    pub fields: Vec<ObjectField>,
    pub methods: Vec<ObjectMethod>,
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub span: Span,
    pub visibility: VisibilityModifier,
    pub external: bool,
}
#[derive(Debug)]
pub struct ComponentDeclaration {
    pub name: Spanned<DedupPoolId<GenericIdentifier>>,
    pub members: Vec<ComponentMember>,
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub visibility: VisibilityModifier,
    pub span: Span,
}
#[derive(Debug)]
pub struct FuncDeclaration {
    pub name: Spanned<DedupPoolId<GenericIdentifier>>,
    pub args: Vec<Spanned<TypedName>>,
    pub return_type: Spanned<DedupPoolId<GenericIdentifier>>,
    pub body: Vec<Spanned<DedupPoolId<ASTStatement>>>,
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub visibility: VisibilityModifier,
    pub span: Span,
    pub external: bool,
}
#[derive(Debug)]
pub struct StyleSheet {
    pub name: Spanned<DedupPoolId<GenericIdentifier>>,
    pub args: Vec<Spanned<TypedName>>,
    pub usages: Vec<Spanned<DedupPoolId<ASTExpression>>>,
    pub body: Vec<Spanned<StyleSheetStatement>>,
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub visibility: VisibilityModifier,
    pub span: Span,
}
#[derive(Debug)]
pub struct StaticDeclaration {
    pub name: SymbolPointer,
    pub ty: Spanned<DedupPoolId<GenericIdentifier>>,
    pub value: Option<Spanned<DedupPoolId<ASTExpression>>>, //option because, if not provided, it yet can be used, even though might lead to runtime bugs. Should be None only on externs
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub visibility: VisibilityModifier,
    pub span: Span,
    pub external: bool,
}

#[derive(Debug)]
pub struct StyleState {
    pub states: Vec<SymbolPointer>,
    pub duration: Option<Spanned<DedupPoolId<ASTExpression>>>,
    pub transition_curve: Option<SymbolPointer>,
}

impl Default for StyleState {
    fn default() -> Self {
        Self::new()
    }
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
