use common::{Span, VisibilityModifier};

use crate::{
    ASTExpression, ASTExpressionKind, ASTStatement, ComponentMember, GenericIdentifier,
    ObjectField, StyleSheetStatement, SymbolPointer, TypedName,
};

#[derive(Debug)]
///Represents a @name(...args). An Attribute is mainly used to define some metadata about the given declaration.
pub struct ASTAttribute {
    pub name: SymbolPointer,
    pub args: Vec<SymbolPointer>,
}

#[derive(Debug)]
pub struct ObjectMethod {
    pub method_name: GenericIdentifier,
    pub arguments: Vec<TypedName>,
    pub return_type: GenericIdentifier,
    pub body: Vec<ASTStatement>,
    pub span: Span,
}

#[derive(Debug)]
pub struct AliasDeclaration {
    pub name: GenericIdentifier,
    pub target: GenericIdentifier,
    pub span: Span,
}
#[derive(Debug)]
pub struct ObjectDeclaration {
    pub name: GenericIdentifier,
    pub fields: Vec<ObjectField>,
    pub methods: Vec<ObjectMethod>,
    pub attributes: Vec<ASTAttribute>,
    pub span: Span,
    pub visibility: VisibilityModifier,
    pub external: bool,
}
#[derive(Debug)]
pub struct ComponentDeclaration {
    pub name: GenericIdentifier,
    pub members: Vec<ComponentMember>,
    pub attributes: Vec<ASTAttribute>,
    pub visibility: VisibilityModifier,
    pub span: Span,
}
#[derive(Debug)]
pub struct FuncDeclaration {
    pub name: GenericIdentifier,
    pub args: Vec<TypedName>,
    pub return_type: GenericIdentifier,
    pub body: Vec<ASTStatement>,
    pub attributes: Vec<ASTAttribute>,
    pub visibility: VisibilityModifier,
    pub span: Span,
    pub external: bool,
}
pub struct StyleSheet {
    pub name: GenericIdentifier,
    pub args: Vec<TypedName>,
    pub usages: Vec<ASTExpression>,
    pub body: Vec<StyleSheetStatement>,
    pub attributes: Vec<ASTAttribute>,
    pub visibility: VisibilityModifier,
    pub span: Span,
}
#[derive(Debug)]
pub struct StaticDeclaration {
    pub name: SymbolPointer,
    pub ty: GenericIdentifier,
    pub value: Option<ASTExpression>, //option because, if not provided, it yet can be used, even though might lead to runtime bugs. Should be None only on externs
    pub attributes: Vec<ASTAttribute>,
    pub visibility: VisibilityModifier,
    pub span: Span,
}

impl ASTExpression {
    pub fn is_assignable(&self) -> bool {
        matches!(
            self.kind,
            ASTExpressionKind::Identifier(_) | ASTExpressionKind::FieldAccess { .. },
        )
    }
}

#[derive(Debug)]
pub struct StyleState {
    pub states: Vec<SymbolPointer>,
    pub duration: Option<ASTExpression>,
    pub transition_curve: Option<SymbolPointer>,
}
