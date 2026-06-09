use common::{Span, VisibilityModifier};

use crate::{
    ASTExpression, ASTExpressionKind, ASTStatement, ComponentMember, FileImport, GenericIdentifier,
    StyleSheetStatement, TypedName,
};

#[derive(Debug)]
///Represents a @name(...args). An Attribute is mainly used to define some metadata about the given declaration.
pub struct ASTAttribute {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug)]
pub struct ASTDeclaration {
    pub attributes: Vec<ASTAttribute>,
    pub visibility: VisibilityModifier,
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
    Import(FileImport),

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
    StyleSheet {
        name: GenericIdentifier,
        args: Vec<TypedName>,
        usages: Vec<ASTExpression>,
        body: Vec<StyleSheetStatement>,
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

#[derive(Debug)]
pub struct StyleState {
    pub states: Vec<String>,
    pub duration: Option<ASTExpression>,
    pub transition_curve: Option<String>,
}
impl StyleState {
    ///Creates a style state which represents the base state of the style
    pub fn new_base() -> Self {
        Self {
            states: vec!["default".to_string()],
            duration: None,
            transition_curve: None,
        }
    }
}
