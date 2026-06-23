use crate::{DeclarationId, HirType, SymbolPointer};

mod declarations;
mod lang_items;
mod scopes;
mod symbols;
mod types;
use common::pool::PoolId;
pub use declarations::*;
pub use lang_items::*;
pub use scopes::*;
pub use symbols::*;
pub use types::*;

pub struct DeclarationInfo {
    pub id: DeclarationId,
    pub ty: PoolId<HirType>,
    pub symbol: SymbolPointer,
}
