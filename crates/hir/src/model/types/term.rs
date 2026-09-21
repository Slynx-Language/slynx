use std::sync::Arc;

use common::pool::DedupPoolId;

use crate::{DescriptorId, SymbolPointer};

pub type TermId = DedupPoolId<Term>;

///A primitive type on the HIR. This represents types that are primitive and cannot be defined by the user.
pub enum PrimitiveType {
    ///An Unsigned integer with the given `bitsize` size in bits
    Unsigned { bitsize: u8 },
    ///An Signed integer with the given `bitsize` size in bits
    Signed { bitsize: u8 },
    ///A float 32 value
    Float32,
    ///A float 64 value
    Float64,
    ///A void value
    Void,
    ///A string value
    String,
}

impl PrimitiveType {
    pub fn boolean_type() -> Self {
        Self::Unsigned { bitsize: 1 }
    }
}

pub enum TermStage {
    Runtime,
    CompileTime,
}

pub struct VarTerm {
    ///The name of the variable term. If given by func f<T>(x: T), then the name is 'T'
    pub name: SymbolPointer,
    ///The index this term appears on a given context
    pub index: u8,
}

pub type HoleId = DedupPoolId<()>;

pub enum TermNode {
    ///Represents a primitive type on the language
    Primitive(PrimitiveType),
    ///Represents a var type on the language. In common terms, generic types
    Var(VarTerm),

    ///A node that represents an application of a term with others, such as Struct<G,E,N> to represent a generic struct
    Apply {
        target: TermId,
        args: Vec<TermId>,
    },
    ///A tuple node
    Tuple {
        fields: Vec<TermId>,
    },
    ///A node that represents a type
    Func {
        args: Vec<TermId>,
        ret: TermId,
    },
    ///A node that represents a referece to a type.
    Ref {
        mutable: bool,
        target: TermId,
    },

    ///Represents something that was defined by user on the language, such as
    Data(DescriptorId),
    Hole(HoleId),
    Extension(Arc<dyn ExtensionNode>),
}

pub enum TermKind {
    Type,
    Kind(DedupPoolId<TermKind>, DedupPoolId<TermKind>),
    Universe,
}

pub struct Term {
    node: TermNode,
    stage: TermStage,
    kind: TermKind,
}

pub trait ExtensionNode: std::fmt::Debug + std::any::Any {
    fn children(&self) -> &[TermId];
    fn kind(&self) -> TermKind;
    fn name(&self) -> &'static str;
    fn dyn_eq(&self, other: &dyn ExtensionNode) -> bool;
    fn dyn_hash(&self, state: &mut dyn std::hash::Hasher);
    fn as_any(&self) -> &dyn std::any::Any;
}

impl std::hash::Hash for dyn ExtensionNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}
