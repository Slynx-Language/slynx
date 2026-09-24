use std::sync::Arc;

use common::pool::DedupPoolId;
use module_loader::ASTBuiltin;

use crate::{
    ComponentType, DescriptorId, EnumType, Result, StructType, SymbolPointer,
    generic_component::GenericComponentTerm,
};

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
    pub const fn const_usize_type(size: usize) -> Self {
        Self::new_type(TermNode::Constant(ConstantTerm::Usize(size)))
    }

    pub fn extension_type<Ext: ExtensionNode>(ext: Ext) -> Self {
        Self::new_type(TermNode::Extension(Arc::new(ext)))
    }

    pub const fn extension(ext: Arc<dyn ExtensionNode>) -> Self {
        Self::new_type(TermNode::Extension(ext))
    }

    pub const fn application(target: TermId, args: Vec<TermId>) -> Self {
        Self::new_type(TermNode::Apply { target, args })
    }

    pub const fn reference(target: TermId) -> Self {
        Self::new_type(TermNode::Ref {
            mutable: false,
            target,
        })
    }

    pub const fn mutable_reference(target: TermId) -> Self {
        Self::new_type(TermNode::Ref {
            mutable: true,
            target,
        })
    }

    pub const fn component_type(component: DedupPoolId<ComponentType>) -> Self {
        Self::new_type(TermNode::Data(DescriptorId::Component(component)))
    }
    pub const fn enum_type(enumt: DedupPoolId<EnumType>) -> Self {
        Self::new_type(TermNode::Data(DescriptorId::Enum(enumt)))
    }
    pub const fn struct_type(strukt: DedupPoolId<StructType>) -> Self {
        Self::new_type(TermNode::Data(DescriptorId::Struct(strukt)))
    }
    pub fn generic_component_type() -> Self {
        Self::extension_type(GenericComponentTerm)
    }

    pub const fn var_type(index: u8, name: SymbolPointer) -> Self {
        Self::new_type(TermNode::Var(VarTerm { index, name }))
    }

    pub const fn string_type() -> Self {
        Self::new_type(TermNode::Primitive(PrimitiveType::String))
    }

    pub const fn float32_type() -> Self {
        Self::new_type(TermNode::Primitive(PrimitiveType::Float32))
    }

    pub const fn float64_type() -> Self {
        Self::new_type(TermNode::Primitive(PrimitiveType::Float64))
    }

    pub const fn signed_integer_type(bitsize: u8) -> Self {
        Self::new_type(TermNode::Primitive(PrimitiveType::Signed { bitsize }))
    }

    pub const fn unsigned_integer_type(bitsize: u8) -> Self {
        Self::new_type(TermNode::Primitive(PrimitiveType::Unsigned { bitsize }))
    }

    pub const fn boolean_type() -> Self {
        Self::unsigned_integer_type(1)
    }

    pub const fn void_type() -> Self {
        Self::new_type(TermNode::Primitive(PrimitiveType::Void))
    }
    pub const fn tuple_type(fields: Vec<TermId>) -> Self {
        Self::new_type(TermNode::Tuple { fields })
    }
    pub const fn function_type(args: Vec<TermId>, ret: TermId) -> Self {
        Self::new_type(TermNode::Func { args, ret })
    }

    ///Creates a new variable type term
    pub const fn new_variable_type(index: u8, name: SymbolPointer) -> Self {
        Self::new_type(TermNode::Var(VarTerm { index, name }))
    }

    pub const fn new_type(node: TermNode) -> Self {
        Self::new(node, TermStage::Runtime, TermKind::Type)
    }

    pub const fn new_runtime(node: TermNode, kind: TermKind) -> Self {
        Self::new(node, TermStage::Runtime, kind)
    }

    pub const fn new(node: TermNode, stage: TermStage, kind: TermKind) -> Self {
        Self { node, stage, kind }
    }
}

impl Term {
    /// Returns the bit size of this term, if it represents an unsigned integer type.
    pub fn is_unsigned(&self) -> Option<u8> {
        if let TermNode::Primitive(PrimitiveType::Unsigned { bitsize }) = self.node() {
            Some(*bitsize)
        } else {
            None
        }
    }

    pub fn is_signed(&self) -> Option<u8> {
        if let TermNode::Primitive(PrimitiveType::Signed { bitsize }) = self.node() {
            Some(*bitsize)
        } else {
            None
        }
    }
    pub fn is_primitive(&self) -> Option<PrimitiveType> {
        if let TermNode::Primitive(p) = self.node() {
            Some(p.clone())
        } else {
            None
        }
    }

    pub fn is_var_type(&self) -> Option<&VarTerm> {
        match self.node() {
            TermNode::Var(term) => Some(term),
            _ => None,
        }
    }
    pub const fn node(&self) -> &TermNode {
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
    fn try_map_children(
        &self,
        f: &mut dyn FnMut(TermId) -> Result<TermId>,
    ) -> Result<Arc<dyn ExtensionNode>>;
    fn dyn_eq(&self, other: &dyn ExtensionNode) -> bool;
    fn dyn_hash(&self, state: &mut dyn std::hash::Hasher);
    fn as_any(&self) -> &dyn std::any::Any;
}

impl std::hash::Hash for dyn ExtensionNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}
impl From<ASTBuiltin> for Term {
    fn from(value: ASTBuiltin) -> Self {
        match value {
            ASTBuiltin::Boolean => Self::boolean_type(),
            ASTBuiltin::F16 | ASTBuiltin::F32 => Self::float32_type(),
            ASTBuiltin::F64 => Self::float64_type(),
            ASTBuiltin::Int(n) => Self::signed_integer_type(n),
            ASTBuiltin::Uint(n) => Self::unsigned_integer_type(n),
            ASTBuiltin::Void => Self::void_type(),
            ASTBuiltin::Str => Self::string_type(),
            ASTBuiltin::AnyComponent => Self::generic_component_type(),
        }
    }
}
