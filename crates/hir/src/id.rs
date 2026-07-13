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
        *self
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

/// Type alias for a function declaration ID.
pub type FunctionId = DeclarationId<HirFunctionDeclaration>;

/// Type alias for a component declaration ID.
pub type ComponentId = DeclarationId<HirComponentDeclaration>;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ExpressionId {
    owner: OwnerId,
    index: DedupPoolId<HirExpression>,
}

impl ExpressionId {
    pub fn new(owner: OwnerId, index: DedupPoolId<HirExpression>) -> Self {
        Self { owner, index }
    }
}

/// Identifies the owner of a variable — either a function or a component.
///
/// This allows a single [`VariableId`] type to be used for local variables
/// in function bodies as well as property slots in component bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwnerId {
    /// The variable belongs to a function's local scope.
    Function(FunctionId),
    /// The variable belongs to a component's property scope.
    Component(ComponentId),
}

/// Uniquely identifies a variable within its owner scope.
///
/// # Owner types
///
/// - [`OwnerId::Function`] — local variables in function bodies
/// - [`OwnerId::Component`] — property bindings in component bodies
///
/// # Index
///
/// The `index` field distinguishes multiple variables within the same owner.
/// For functions this is the argument or local variable position; for components
/// it is the property index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariableId {
    pub owner: OwnerId,
    pub index: u16,
}

impl VariableId {
    /// Creates a new `VariableId` owned by a function declaration.
    #[inline]
    pub fn new(owner: OwnerId, index: u16) -> Self {
        Self { owner, index }
    }

    /// Creates a new `VariableId` owned by a function declaration.
    #[inline]
    pub fn function(fid: FunctionId, index: u16) -> Self {
        Self {
            owner: OwnerId::Function(fid),
            index,
        }
    }

    /// Creates a new `VariableId` owned by a component declaration.
    #[inline]
    pub fn component(cid: ComponentId, index: u16) -> Self {
        Self {
            owner: OwnerId::Component(cid),
            index,
        }
    }

    /// Returns `true` if this variable is owned by a function.
    #[inline]
    pub fn is_function(&self) -> bool {
        matches!(self.owner, OwnerId::Function(_))
    }

    /// Returns `true` if this variable is owned by a component.
    #[inline]
    pub fn is_component(&self) -> bool {
        matches!(self.owner, OwnerId::Component(_))
    }

    /// Unwraps the inner [`FunctionId`], panicking if this is not a function-owned variable.
    #[inline]
    pub fn unwrap_function(&self) -> FunctionId {
        match self.owner {
            OwnerId::Function(fid) => fid,
            _ => panic!("VariableId is not a function variable"),
        }
    }

    /// Unwraps the inner [`ComponentId`], panicking if this is not a component-owned variable.
    #[inline]
    pub fn unwrap_component(&self) -> ComponentId {
        match self.owner {
            OwnerId::Component(cid) => cid,
            _ => panic!("VariableId is not a component variable"),
        }
    }
}
