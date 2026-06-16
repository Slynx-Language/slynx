use std::default;

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
pub struct ObjectField {
    pub visibility: VisibilityModifier,
    pub name: TypedName,
}

#[derive(Debug)]
pub struct ObjectMethod {
    pub method_name: GenericIdentifier,
    pub arguments: Vec<TypedName>,
    pub return_type: GenericIdentifier,
    pub body: Vec<ASTStatement>,
    pub span: Span,
}

impl ObjectMethod {
    pub fn is_static(&self) -> bool {
        if let Some(arg) = self.arguments.first()
            && (arg.kind.identifier == "Self" || arg.kind.identifier == "self")
        {
            false
        } else {
            true
        }
    }
}

#[derive(Debug)]
pub struct ASTDeclaration {
    pub attributes: Vec<ASTAttribute>,
    pub visibility: VisibilityModifier,
    pub external: bool,
    pub kind: ASTDeclarationKind,
    pub span: Span,
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
        methods: Vec<ObjectMethod>,
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
    Static {
        name: String,
        ty: GenericIdentifier,
        value: Option<ASTExpression>, //option because, if not provided, it yet can be used, even though might lead to runtime bugs. Should be None only on externs
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
