use std::hash::Hash;

use common::{Spanned, VisibilityModifier, pool::PoolId};
use smallvec::SmallVec;

use crate::SymbolPointer;

#[derive(Debug)]
///A name that is typed. This is simply the representation of `name: kind`
pub struct TypedName {
    pub name: SymbolPointer,
    pub kind: Spanned<PoolId<GenericIdentifier>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
///A Identifier that might contain a generic. Such as `Component<int>`
pub struct GenericIdentifier {
    ///The generic this identifier contains.
    pub generic: SmallVec<[PoolId<GenericIdentifier>; 2]>,
    ///The name of this identifier
    pub identifier: SymbolPointer,
}
#[derive(Debug)]
pub struct ObjectField {
    pub visibility: VisibilityModifier,
    pub name: Spanned<TypedName>,
}
