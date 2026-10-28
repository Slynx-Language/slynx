use dashmap::DashMap;

use crate::{HirCallbackDefinition, PropertyCallback, SymbolPointer, term::TermId};

/// Type-level callback contracts and component-level callback implementations.
///
/// Callbacks deliberately live outside [`MethodTable`](super::MethodTable): they
/// are call-site instrumentation metadata, not methods. Keeping them in a
/// separate table guarantees they never become source-reachable through method
/// lookup.
#[derive(Debug, Default)]
pub struct CallbackTable {
    definitions: DashMap<(TermId, SymbolPointer), Vec<HirCallbackDefinition>>,
    implementations: DashMap<(TermId, usize), Vec<PropertyCallback>>,
}

impl CallbackTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the contracts attached to `owner`'s method `method`.
    pub fn set_definitions(
        &self,
        owner: TermId,
        method: SymbolPointer,
        definitions: Vec<HirCallbackDefinition>,
    ) {
        self.definitions.insert((owner, method), definitions);
    }

    /// Returns the contracts attached to `owner`'s method `method`.
    pub fn definitions_of(
        &self,
        owner: TermId,
        method: SymbolPointer,
    ) -> Vec<HirCallbackDefinition> {
        self.definitions
            .get(&(owner, method))
            .map(|definitions| definitions.clone())
            .unwrap_or_default()
    }

    /// Finds a callback contract by its declared name on `owner`.
    pub fn definition_named(
        &self,
        owner: TermId,
        callback: SymbolPointer,
    ) -> Option<HirCallbackDefinition> {
        self.definitions
            .iter()
            .filter(|entry| entry.key().0 == owner)
            .find_map(|entry| {
                entry
                    .value()
                    .iter()
                    .find(|definition| definition.name == callback)
                    .cloned()
            })
    }

    /// Registers the callback implementations attached to a component property.
    pub fn set_property_callbacks(
        &self,
        component: TermId,
        property_index: usize,
        callbacks: Vec<PropertyCallback>,
    ) {
        self.implementations
            .insert((component, property_index), callbacks);
    }

    /// Returns the callback implementations attached to a component property.
    pub fn property_callbacks_of(
        &self,
        component: TermId,
        property_index: usize,
    ) -> Vec<PropertyCallback> {
        self.implementations
            .get(&(component, property_index))
            .map(|callbacks| callbacks.clone())
            .unwrap_or_default()
    }
}
