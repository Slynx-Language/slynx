use std::sync::Arc;

use common::pool::DedupPoolId;

use crate::{DescriptorId, SymbolPointer};

pub type TermId = DedupPoolId<Term>;
pub type HoleId = DedupPoolId<()>;

///A primitive type on the HIR. This represents types that are primitive and cannot be defined by the user.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
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

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub enum TermStage {
    #[default]
    Runtime,
    CompileTime,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct VarTerm {
    ///The name of the variable term. If given by func f<T>(x: T), then the name is 'T'
    pub name: SymbolPointer,
    ///The index this term appears on a given context
    pub index: u8,
}

impl VarTerm {
    pub fn new(name: SymbolPointer, index: u8) -> Self {
        Self { name, index }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ConstantTerm {
    Usize(usize),
}

#[derive(Debug, Clone)]
pub enum TermNode {
    Constant(ConstantTerm),
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

impl PartialEq for TermNode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TermNode::Primitive(p1), TermNode::Primitive(p2)) => p1 == p2,
            (TermNode::Var(v1), TermNode::Var(v2)) => v1.index == v2.index,
            (
                TermNode::Func {
                    args: arg1,
                    ret: ret1,
                },
                TermNode::Func {
                    args: arg2,
                    ret: ret2,
                },
            ) => arg1 == arg2 && ret1 == ret2,
            (
                TermNode::Apply { target: t1, args },
                TermNode::Apply {
                    target: t2,
                    args: args2,
                },
            ) => t1 == t2 && args == args2,
            (TermNode::Tuple { fields: f1 }, TermNode::Tuple { fields: f2 }) => f1 == f2,
            (
                TermNode::Ref {
                    mutable: m1,
                    target: t1,
                },
                TermNode::Ref {
                    mutable: m2,
                    target: t2,
                },
            ) => m1 == m2 && t1 == t2,
            (TermNode::Data(d1), TermNode::Data(d2)) => d1 == d2,
            (TermNode::Hole(h1), TermNode::Hole(h2)) => h1 == h2,
            (TermNode::Extension(e1), TermNode::Extension(e2)) => e1.dyn_eq(&**e2),
            (TermNode::Constant(c1), TermNode::Constant(c2)) => c1 == c2,
            _ => false,
        }
    }
}
impl Eq for TermNode {}
impl std::hash::Hash for TermNode {
    fn hash<F: std::hash::Hasher>(&self, state: &mut F) {
        match self {
            Self::Primitive(p) => p.hash(state),
            Self::Var(v) => v.hash(state),
            Self::Apply { target, args } => {
                target.hash(state);
                args.hash(state);
            }
            Self::Tuple { fields } => fields.hash(state),
            Self::Func { args, ret } => {
                args.hash(state);
                ret.hash(state);
            }
            Self::Ref { mutable, target } => {
                mutable.hash(state);
                target.hash(state);
            }
            Self::Data(d) => d.hash(state),
            Self::Hole(h) => h.hash(state),
            Self::Extension(e) => e.dyn_hash(state),
            Self::Constant(c) => c.hash(state),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum TermKind {
    Type,
    Kind(DedupPoolId<TermKind>, DedupPoolId<TermKind>),
    Universe,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Term {
    node: TermNode,
    stage: TermStage,
    kind: TermKind,
}

impl Term {
    pub fn new_type(node: TermNode) -> Self {
        Self::new(node, TermStage::Runtime, TermKind::Type)
    }

    pub fn new_runtime(node: TermNode, kind: TermKind) -> Self {
        Self::new(node, TermStage::Runtime, kind)
    }

    pub fn new(node: TermNode, stage: TermStage, kind: TermKind) -> Self {
        Self { node, stage, kind }
    }

    pub fn node(&self) -> &TermNode {
        &self.node
    }

    pub fn children(&self) -> Vec<TermId> {
        match &self.node {
            TermNode::Apply { target, args } => {
                let mut out = Vec::with_capacity(args.len() + 1);
                out.push(*target);
                out.extend(args);
                out
            }
            TermNode::Tuple { fields } => fields.clone(),

            TermNode::Func { args, ret } => {
                let mut out = Vec::with_capacity(args.len() + 1);
                out.extend(args);
                out.push(*ret);
                out
            }
            TermNode::Ref { target, .. } => vec![*target],
            TermNode::Data(_)
            | TermNode::Hole(_)
            | TermNode::Primitive(_)
            | TermNode::Var(_)
            | TermNode::Constant(_) => {
                vec![]
            }
            TermNode::Extension(ext) => ext.children(),
        }
    }
}

pub trait ExtensionNode: std::fmt::Debug + std::any::Any {
    fn children(&self) -> Vec<TermId>;
    fn kind(&self) -> TermKind;
    fn name(&self) -> &'static str;
    fn map_children(&self, f: &mut dyn FnMut(TermId) -> TermId) -> Arc<dyn ExtensionNode>;
    fn dyn_eq(&self, other: &dyn ExtensionNode) -> bool;
    fn dyn_hash(&self, state: &mut dyn std::hash::Hasher);
    fn as_any(&self) -> &dyn std::any::Any;
}

impl std::hash::Hash for dyn ExtensionNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}
