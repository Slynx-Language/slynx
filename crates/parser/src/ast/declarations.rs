use common::{Span, Spanned, VisibilityModifier, pool::DedupPoolId};
use smallvec::SmallVec;

use crate::{
    ASTExpression, ASTStatement, ComponentMember, ObjectField, StyleSheetStatement, SymbolPointer,
    Type, TypedName,
};

#[derive(Debug)]
///Represents a @name(...args). An Attribute is mainly used to define some metadata about the given declaration.
pub struct ASTAttribute {
    pub name: SymbolPointer,
    pub args: Vec<SymbolPointer>,
}

#[derive(Debug)]
pub struct ObjectMethod {
    ///The type parameters declared by this generic function. Each parameter is a
    ///`Type` (e.g. `Plain("T")`), so that use sites may later substitute any
    ///type, such as `[4]int`, for it.
    pub type_params: Vec<SymbolPointer>,
    pub method_name: SymbolPointer,
    pub arguments: Vec<Spanned<TypedName>>,
    pub return_type: Spanned<DedupPoolId<Type>>,
    pub body: Vec<Spanned<DedupPoolId<ASTStatement>>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct AliasDeclaration {
    ///The type parameters declared by this generic function. Each parameter is a
    ///`Type` (e.g. `Plain("T")`), so that use sites may later substitute any
    ///type, such as `[4]int`, for it.
    pub type_params: Vec<SymbolPointer>,
    pub name: SymbolPointer,
    pub target: Spanned<DedupPoolId<Type>>,
    pub span: Span,
    pub visibility: VisibilityModifier,
}
#[derive(Debug)]
pub struct ObjectDeclaration {
    ///The type parameters declared by this generic function. Each parameter is a
    ///`Type` (e.g. `Plain("T")`), so that use sites may later substitute any
    ///type, such as `[4]int`, for it.
    pub type_params: Vec<SymbolPointer>,
    pub name: SymbolPointer,
    pub fields: Vec<ObjectField>,
    pub methods: Vec<ObjectMethod>,
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub span: Span,
    pub visibility: VisibilityModifier,
    pub external: bool,
}
#[derive(Debug)]
pub struct ComponentDeclaration {
    ///The type parameters declared by this generic function. Each parameter is a
    ///`Type` (e.g. `Plain("T")`), so that use sites may later substitute any
    ///type, such as `[4]int`, for it.
    pub type_params: Vec<SymbolPointer>,
    pub name: SymbolPointer,
    pub members: Vec<ComponentMember>,
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub visibility: VisibilityModifier,
    pub span: Span,
}
#[derive(Debug)]
pub struct FuncDeclaration {
    pub name: SymbolPointer,
    ///The type parameters declared by this generic function. Each parameter is a
    ///`Type` (e.g. `Plain("T")`), so that use sites may later substitute any
    ///type, such as `[4]int`, for it.
    pub type_params: Vec<SymbolPointer>,
    pub args: Vec<Spanned<TypedName>>,
    pub return_type: Spanned<DedupPoolId<Type>>,
    pub body: Vec<Spanned<DedupPoolId<ASTStatement>>>,
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub visibility: VisibilityModifier,
    pub span: Span,
    pub external: bool,
}
#[derive(Debug)]
pub struct StyleSheet {
    ///The type parameters declared by this generic function. Each parameter is a
    ///`Type` (e.g. `Plain("T")`), so that use sites may later substitute any
    ///type, such as `[4]int`, for it.
    pub type_params: Vec<SymbolPointer>,
    pub name: SymbolPointer,
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
    pub ty: Spanned<DedupPoolId<Type>>,
    pub value: Option<Spanned<DedupPoolId<ASTExpression>>>, //option because, if not provided, it yet can be used, even though might lead to runtime bugs. Should be None only on externs
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub visibility: VisibilityModifier,
    pub span: Span,
    pub external: bool,
}

#[derive(Debug)]
pub enum EnumVariantKind {
    ///represents a raw variant, such as 'Name' in `enum N {Name}`
    Raw,
    ///represents a raw valued variant, such as 'Name' in `enum N {Name = 5}`
    RawValued(Spanned<DedupPoolId<ASTExpression>>),
    ///represents an associated variant, such as 'Name' in `enum N {Name(int)}`
    Associated(SmallVec<[Spanned<DedupPoolId<Type>>; 2]>),
    ///represents a struct variant, such as 'Name' in `enum N {Name { <fields> }}`
    Struct(SmallVec<[Spanned<TypedName>; 2]>),
}

#[derive(Debug)]
pub struct EnumVariant {
    ///Name of the variant
    pub name: Spanned<SymbolPointer>,
    ///The kind of the variant
    pub kind: EnumVariantKind,
    ///The attributes of the variant
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct EnumDeclaration {
    ///The name of the enum
    pub name: SymbolPointer,
    ///The generic parameters given to this enum
    pub type_params: Vec<SymbolPointer>,
    ///What representation to use for this enum. Only valid with all variants being Raw or RawValued
    pub representation: Option<Spanned<DedupPoolId<Type>>>,
    pub variants: Vec<EnumVariant>,
    pub attributes: Vec<Spanned<ASTAttribute>>,
    pub visibility: VisibilityModifier,
    pub span: Span,
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
