use std::hash::Hash;

use common::{Spanned, VisibilityModifier, pool::DedupPoolId};
use smallvec::SmallVec;

use crate::{ASTExpression, SymbolPointer};

#[derive(Debug)]
///A name that is typed. This is simply the representation of `name: kind`
pub struct TypedName {
    pub name: SymbolPointer,
    pub kind: Spanned<DedupPoolId<Type>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Type {
    Plain(GenericIdentifier),
    Array(DedupPoolId<Type>, DedupPoolId<ASTExpression>),
    Vector(DedupPoolId<Type>),
    Nullable(DedupPoolId<Type>),
    ///A reference to the i-th type parameter of the enclosing generic declaration.
    ///For example, in `func A<T>(arg: T): T`, both the `arg` type and the return
    ///type are represented as `Generic(0)`.
    Generic(u8),
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
///A Identifier that might contain a generic. Such as `Component<int>`
pub struct GenericIdentifier {
    ///The generic this identifier contains.
    pub generic: SmallVec<[Spanned<DedupPoolId<Type>>; 2]>,
    ///The name of this identifier
    pub identifier: SymbolPointer,
}
#[derive(Debug)]
pub struct ObjectField {
    pub visibility: VisibilityModifier,
    pub name: Spanned<TypedName>,
}
