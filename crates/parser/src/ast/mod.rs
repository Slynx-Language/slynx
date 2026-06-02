mod component;
mod expression;
mod types;

use common::Span;
pub use component::*;
pub use expression::*;
pub use types::*;
#[derive(Default, Debug, Clone, Copy)]
pub enum VisibilityModifier {
    ///Property visible to everyone
    Public,
    ///Property visible only by the one defining it.
    #[default]
    Private,
    ///Property visible only for the children. Only usable on Components.
    ChildrenPublic,
    ///Property visible only for the parents. Only usable on Components.
    ParentPublic,
}

#[derive(Debug)]
///Some statement on the code, a statement not necessarily have value, in general expressions do.
pub struct ASTStatement {
    pub kind: ASTStatementKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ASTStatementKind {
    Var {
        name: String,
        ty: Option<GenericIdentifier>,
        rhs: ASTExpression,
    },
    MutableVar {
        name: String,
        ty: Option<GenericIdentifier>,
        rhs: ASTExpression,
    },
    Assign {
        ///The Left hand side of the assign, or, the one that will receive the value of `rhs`
        lhs: ASTExpression,
        rhs: ASTExpression,
    },

    While {
        condition: ASTExpression,
        body: Vec<ASTStatement>,
    },

    Expression(ASTExpression),
}

#[derive(Debug)]
pub struct ASTDeclaration {
    pub kind: ASTDeclarationKind,
    pub span: Span,
}

#[derive(Debug)]
pub struct ObjectField {
    pub visibility: VisibilityModifier,
    pub name: TypedName,
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
#[derive(Debug)]
pub struct StyleBlock {
    pub state: StyleState,
    pub properties: Vec<NamedExpr>,
    pub children: Vec<StyleBlock>,
    pub span: Span,
}

#[derive(Debug)]
pub enum StyleSheetStatement {
    Statement(Box<ASTStatement>),
    Styles { styles: Vec<StyleBlock>, span: Span },
}

#[derive(Debug)]
pub enum ASTDeclarationKind {
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
    EnumDeclaration{
        name: GenericIdentifier,
        variants: Vec<String>,
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
