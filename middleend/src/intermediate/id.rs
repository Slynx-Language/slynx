use std::sync::atomic::{AtomicU64, Ordering};

/// Shared trait for all HIR IDs
/// Ensures all IDs have consistent behavior
pub trait IntermediateIdTrait:
    Copy + Clone + std::fmt::Debug + std::hash::Hash + Eq + PartialEq
{
    fn as_u64(&self) -> u64;
    fn from_u64(value: u64) -> Self;
}

/// Macro to generate newtype wrappers for IDs with standard behavior
macro_rules! define_intermediate_id {
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

        impl IntermediateIdTrait for $name {
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

#[derive(Debug, Clone, Copy)]
pub struct ContextHandle(pub usize);

define_intermediate_id!(
    PropId,
    PROPERTY_COUNTER,
    "Unique ID for top-level declarations (functions, components, objects)"
);
define_intermediate_id!(
    DeclId,
    DECLARATION_COUNTER,
    "Unique ID for top-level declarations (functions, components, objects)"
);

//Maybe child nodes might receive an specific id, but until now i dont see a reason to do so
define_intermediate_id!(
    ValueId,
    EXPRESSION_COUNTER,
    "Unique ID for expressions (child nodes included)"
);

define_intermediate_id!(
    VarId,
    VARIABLE_COUNTER,
    "Unique ID for variables (let/let mut)"
);

define_intermediate_id!(
    TyId,
    TYPE_COUNTER,
    "Unique ID for custom types (structs, objects, components)"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_uniqueness() {
        let id1 = DeclId::new();
        let id2 = DeclId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_id_ordering() {
        let id1 = ValueId::new();
        let id2 = ValueId::new();
        assert!(id1 < id2);
    }

    #[test]
    fn test_id_raw_conversion() {
        let id = VarId::new();
        let raw = id.as_raw();
        let reconstructed = VarId::from_raw(raw);
        assert_eq!(id, reconstructed);
    }
}
