use std::fmt::Debug;

use crate::{StyleType, SymbolPointer};
use common::dedup_pooled;
use common::pool::{DedupPool, DedupPoolId};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct StyleMetadata {
    pub(crate) name: SymbolPointer,
}

dedup_pooled!(pub StylesPool {
    styles: StyleType,
    metadata: StyleMetadata
});

impl Debug for StylesPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StylesPool")
            .field("styles", &self.styles)
            .field("metadata", &self.metadata)
            .finish()
    }
}
