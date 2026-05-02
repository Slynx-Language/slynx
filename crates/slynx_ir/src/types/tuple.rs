use crate::IRTypeId;

/// Represents a tuple type in the Intermediate Representation (IR).
///
/// An `IRTuple` is a **composite type** made of an ordered list of element types.
/// Each element is identified by an [`IRTypeId`], allowing the tuple to reference
/// types stored in the central IR type table.
/// This struct represents the **data** of a tuple,
/// while [`IRTupleId`] is used as a lightweight handle (index)
/// into the tuple storage inside `IRTypes`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IRTuple {
    /// Ordered list of element types that make up the tuple.
    pub elements: Vec<IRTypeId>,
}

/// A compact identifier for a tuple type stored in the IR.
///
/// This is essentially an index into the `IRTypes.tuples` storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IRTupleId(pub usize);

impl IRTuple {
    /// Creates a new tuple type from a list of element types.
    ///
    /// # Arguments
    ///
    /// * `elements` - A vector of [`IRTypeId`] representing the types of each element.
    pub fn new(elements: Vec<IRTypeId>) -> Self {
        Self { elements }
    }

    /// Returns a slice of all element types in the tuple.
    /// This is useful for iteration or inspection without cloning.
    pub fn types(&self) -> &[IRTypeId] {
        &self.elements
    }

    /// Returns `true` if the tuple has no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns the number of elements in the tuple.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Returns the type of the element at the given index, if it exists.
    ///
    /// # Arguments
    ///
    /// * `index` - The position of the element in the tuple.
    ///
    /// # Returns
    ///
    /// - `Some(&IRTypeId)` if the index is valid
    /// - `None` if the index is out of bounds
    pub fn get(&self, index: usize) -> Option<&IRTypeId> {
        self.elements.get(index)
    }
}
