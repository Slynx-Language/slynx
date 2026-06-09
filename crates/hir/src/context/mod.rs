use crate::{DeclarationId, SymbolPointer, TypeId};

mod declarations;
mod scopes;
mod symbols;
mod types;
pub use declarations::*;
pub use scopes::*;
pub use symbols::*;
pub use types::*;

pub struct DeclarationInfo {
    pub id: DeclarationId,
    pub ty: TypeId,
    pub symbol: SymbolPointer,
}
