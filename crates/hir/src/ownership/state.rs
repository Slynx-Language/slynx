//! Ownership state tracking for places.

use std::collections::HashMap;

use crate::{DeclarationId, HirStaticDeclaration, VariableId};

/// Whether a borrow is mutable or immutable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Mutable,
    Immutable,
}

impl std::fmt::Display for BorrowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowKind::Mutable => write!(f, "mutable"),
            BorrowKind::Immutable => write!(f, "immutable"),
        }
    }
}

/// The state of a single place in the ownership system.
#[derive(Debug, Clone)]
pub struct PlaceState {
    /// Number of active mutable borrows.
    pub borrowed_mut: u8,
    /// Number of active immutable borrows.
    pub borrowed_immut: u8,
    /// Whether the place has been moved.
    pub moved: bool,
}

impl PlaceState {
    pub fn new() -> Self {
        Self {
            borrowed_mut: 0,
            borrowed_immut: 0,
            moved: false,
        }
    }

    /// Whether this place can be moved from.
    pub fn can_move(&self) -> bool {
        !self.moved && self.borrowed_mut == 0 && self.borrowed_immut == 0
    }

    /// Whether this place can be borrowed with the given kind.
    pub fn can_borrow(&self, kind: BorrowKind) -> bool {
        !self.moved
            && match kind {
                BorrowKind::Mutable => self.borrowed_mut == 0 && self.borrowed_immut == 0,
                BorrowKind::Immutable => self.borrowed_mut == 0,
            }
    }

    /// Whether this place has been moved.
    pub fn is_moved(&self) -> bool {
        self.moved
    }

    /// Whether this place has any active borrows.
    pub fn is_borrowed(&self) -> bool {
        self.borrowed_mut > 0 || self.borrowed_immut > 0
    }
}

/// Ownership state for a function body.
///
/// Tracks the state of all places used in a function, including
/// variables, temporaries, and projections.
#[derive(Debug)]
pub struct FunctionOwnershipState {
    /// State for each variable (by VariableId).
    pub variable_states: HashMap<VariableId, PlaceState>,
    pub static_states: HashMap<DeclarationId<HirStaticDeclaration>, PlaceState>,
}

impl FunctionOwnershipState {
    pub fn new() -> Self {
        Self {
            variable_states: HashMap::new(),
            static_states: HashMap::new(),
        }
    }

    /// Get the state for a variable, creating a default state if not present.
    pub fn get_variable_state(&mut self, id: VariableId) -> &mut PlaceState {
        self.variable_states
            .entry(id)
            .or_insert_with(PlaceState::new)
    }

    pub fn mark_static_moved(&mut self, id: DeclarationId<HirStaticDeclaration>) {
        self.static_states
            .entry(id)
            .or_insert_with(PlaceState::new)
            .moved = true;
    }
    pub fn is_static_moved(&self, id: DeclarationId<HirStaticDeclaration>) -> bool {
        self.static_states
            .get(&id)
            .map(|s| s.moved)
            .unwrap_or(false)
    }

    /// Check if a variable has been moved.
    pub fn is_variable_moved(&self, id: VariableId) -> bool {
        self.variable_states
            .get(&id)
            .map(|s| s.moved)
            .unwrap_or(false)
    }

    /// Mark a variable as moved.
    pub fn mark_variable_moved(&mut self, id: VariableId) {
        let state = self.get_variable_state(id);
        state.moved = true;
    }

    /// Check if a variable can be borrowed with the given kind.
    pub fn can_borrow_variable(&self, id: VariableId, kind: BorrowKind) -> bool {
        self.variable_states
            .get(&id)
            .map(|s| s.can_borrow(kind))
            .unwrap_or(true)
    }

    /// Borrow a variable.
    pub fn borrow_variable(&mut self, id: VariableId, kind: BorrowKind) {
        let state = self.get_variable_state(id);
        match kind {
            BorrowKind::Mutable => state.borrowed_mut += 1,
            BorrowKind::Immutable => state.borrowed_immut += 1,
        }
    }

    /// Release a borrow on a variable.
    pub fn release_borrow(&mut self, id: VariableId, kind: BorrowKind) {
        if let Some(state) = self.variable_states.get_mut(&id) {
            match kind {
                BorrowKind::Mutable => state.borrowed_mut = state.borrowed_mut.saturating_sub(1),
                BorrowKind::Immutable => {
                    state.borrowed_immut = state.borrowed_immut.saturating_sub(1)
                }
            }
        }
    }

    /// Check if a variable can be moved.
    pub fn can_move_variable(&self, id: VariableId) -> bool {
        self.variable_states
            .get(&id)
            .map(|s| s.can_move())
            .unwrap_or(true)
    }
}
