use std::sync::atomic::{AtomicU64, Ordering};

use crate::module_loader::FileId;

/// Shared trait for all HIR IDs
/// Ensures all IDs have consistent behavior
pub trait HirIdTrait: Copy + Clone + std::fmt::Debug + std::hash::Hash + Eq + PartialEq {
    /// Returns the inner `u64` value of this ID.
    fn as_u64(&self) -> u64;
    /// Constructs an ID from a raw `u64` value.
    fn from_u64(value: u64) -> Self;
}
///The local ID for some declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDeclId(pub(crate) u32);
impl LocalDeclId {
    pub fn from_raw(value: u32) -> Self {
        Self(value)
    }
    pub fn as_raw(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct DeclarationId {
    pub file_id: FileId,
    pub local_id: LocalDeclId,
}
impl DeclarationId {
    pub fn new(file_id: FileId, local_id: LocalDeclId) -> Self {
        Self { file_id, local_id }
    }
}

/// Macro to generate newtype wrappers for IDs with standard behavior
macro_rules! define_hir_id {
    ($name:ident, $counter:ident, $doc:expr) => {
        static $counter: AtomicU64 = AtomicU64::new(0);

        #[doc = $doc]
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(u64);

        impl $name {
            /// Creates a new unique ID
            #[inline]
            pub fn new() -> Self {
                Self($counter.fetch_add(1, Ordering::Relaxed))
            }

            /// Creates an ID from a u64 value (used for deserialization)
            #[inline]
            pub fn from_raw(value: u64) -> Self {
                Self(value)
            }

            /// Returns the internal ID value
            #[inline]
            pub fn as_raw(&self) -> u64 {
                self.0
            }
        }

        impl HirIdTrait for $name {
            #[inline]
            fn as_u64(&self) -> u64 {
                self.0
            }

            #[inline]
            fn from_u64(value: u64) -> Self {
                Self(value)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

// Definition of all specific IDs

define_hir_id!(
    SymbolID,
    SYMBOL_COUNTER,
    "Unique ID to intern strings internally"
);

define_hir_id!(
    ExpressionId,
    EXPRESSION_COUNTER,
    "Unique ID for expressions"
);

define_hir_id!(
    VariableId,
    VARIABLE_COUNTER,
    "Unique ID for variables (let/let mut)"
);

define_hir_id!(
    PropertyId,
    PROPERTY_COUNTER,
    "Unique ID for component properties"
);

define_hir_id!(
    TypeId,
    TYPE_COUNTER,
    "Unique ID for custom types (structs, objects, components)"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_ordering() {
        let id1 = ExpressionId::new();
        let id2 = ExpressionId::new();
        assert!(id1 < id2);
    }

    #[test]
    fn test_id_raw_conversion() {
        let id = VariableId::new();
        let raw = id.as_raw();
        let reconstructed = VariableId::from_raw(raw);
        assert_eq!(id, reconstructed);
    }
}
