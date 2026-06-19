use common::VisibilityModifier;

use crate::{SymbolPointer, ast::Span};

#[derive(Debug)]
///A name that is typed. This is simply the representation of `name: kind`
pub struct TypedName {
    pub name: SymbolPointer,
    pub kind: GenericIdentifier,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, Clone)]
///A Identifier that might contain a generic. Such as `Component<int>`
pub struct GenericIdentifier {
    ///The generic this identifier contains.
    pub generic: Option<Vec<GenericIdentifier>>,
    ///The name of this identifier
    pub identifier: SymbolPointer,
    pub span: Span,
}
#[derive(Debug)]
pub struct ObjectField {
    pub visibility: VisibilityModifier,
    pub name: TypedName,
}
