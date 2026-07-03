use std::hash::Hash;

use common::pool::{DedupPoolId, PoolId};
use module_loader::FileId;

use crate::{
    HirAliasDeclaration, HirComponentDeclaration, HirExpression, HirFunctionDeclaration,
    HirObjectDeclaration, HirStaticDeclaration, HirStylesheetDeclaration,
};

/// Shared trait for all HIR IDs
/// Ensures all IDs have consistent behavior
pub trait HirIdTrait: Copy + Clone + std::fmt::Debug + std::hash::Hash + Eq + PartialEq {
    /// Returns the inner `u64` value of this ID.
    fn as_u64(&self) -> u64;
    /// Constructs an ID from a raw `u64` value.
    fn from_u64(value: u64) -> Self;
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum AnyLocalDeclarationId {
    Object(PoolId<HirObjectDeclaration>),
    Function(PoolId<HirFunctionDeclaration>),
    Component(PoolId<HirComponentDeclaration>),
    Style(PoolId<HirStylesheetDeclaration>),
    Alias(PoolId<HirAliasDeclaration>),
    Static(PoolId<HirStaticDeclaration>),
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct AnyDeclarationId {
    pub file_id: FileId,
    pub local_id: AnyLocalDeclarationId,
}

impl From<DeclarationId<HirStylesheetDeclaration>> for AnyDeclarationId {
    fn from(value: DeclarationId<HirStylesheetDeclaration>) -> Self {
        AnyDeclarationId {
            file_id: value.file_id,
            local_id: AnyLocalDeclarationId::Style(value.local_id),
        }
    }
}

impl AnyDeclarationId {
    pub fn new(file_id: FileId, local_id: AnyLocalDeclarationId) -> Self {
        Self { file_id, local_id }
    }
}

#[derive(Debug)]
pub struct DeclarationId<T> {
    ///The id of the file where this declaration was originated
    pub file_id: FileId,
    ///The id on the pools of the file
    pub local_id: PoolId<T>,
}
impl<T> DeclarationId<T> {
    pub fn new(file_id: FileId, local_id: PoolId<T>) -> Self {
        Self { file_id, local_id }
    }
}
impl<T> Clone for DeclarationId<T> {
    fn clone(&self) -> Self {
        Self::new(self.file_id, self.local_id)
    }
}
impl<T> Copy for DeclarationId<T> {}

impl<T> PartialEq for DeclarationId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.file_id == other.file_id && self.local_id == other.local_id
    }
}
impl<T> Eq for DeclarationId<T> {}
impl<T> Hash for DeclarationId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.file_id.hash(state);
        self.local_id.hash(state);
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ExpressionId {
    owner: DeclarationId<HirFunctionDeclaration>,
    index: DedupPoolId<HirExpression>,
}

impl ExpressionId {
    pub fn new(
        owner: DeclarationId<HirFunctionDeclaration>,
        index: DedupPoolId<HirExpression>,
    ) -> Self {
        Self { owner, index }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct VariableId {
    owner: DeclarationId<HirFunctionDeclaration>,
    index: u8,
}

impl VariableId {
    pub fn new(owner: DeclarationId<HirFunctionDeclaration>, index: u8) -> Self {
        Self { owner, index }
    }
}
