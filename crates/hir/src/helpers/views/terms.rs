use std::{fmt, sync::Arc};

use common::pool::DedupPoolId;

use crate::{
    ComponentType, DescriptorId, EnumType, HirViewer, StructType,
    arrays::ArrayTerm,
    term::{ConstantTerm, ExtensionNode, Term, TermId, TermNode, VarTerm},
    vector::VectorTerm,
};

impl<'a> HirViewer<'a, TermId> {
    pub fn raw(&self) -> &'a Term {
        self.hir.types.storage.terms.get(self.data)
    }

    pub fn children(&self) -> Vec<TermId> {
        self.raw().children()
    }

    pub fn internal_name(&self) -> String {
        match self.dereference().raw().node() {
            TermNode::Primitive(pt) => pt.to_string(),
            TermNode::Var(var) => self.hir.get_name(var.name).into(),
            TermNode::Data(descriptor) => match descriptor {
                DescriptorId::Struct(s) => {
                    let name = self.hir.types.storage.get_struct_name(*s);
                    self.hir.get_name(name).to_string()
                }
                DescriptorId::Enum(e) => {
                    let name = self.hir.types.storage.get_enum_name(*e);
                    self.hir.get_name(name).to_string()
                }
                DescriptorId::Component(c) => self.new_with(*c).name().to_string(),
            },
            TermNode::Func { args, ret } => {
                let args = args
                    .iter()
                    .map(|arg| self.new_with(*arg).internal_name())
                    .collect::<Vec<_>>()
                    .join("_");
                let ret = self.new_with(*ret).internal_name();
                format!("{ret}_func_{args}")
            }
            TermNode::Tuple { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| self.new_with(*field).internal_name())
                    .collect::<Vec<_>>()
                    .join("@");
                format!("Tuple_{fields}")
            }
            TermNode::Ref {
                mutable: false,
                target,
            } => format!("Ref_{}", self.new_with(*target).internal_name()),
            TermNode::Ref {
                mutable: true,
                target,
            } => format!("MRef_{}", self.new_with(*target).internal_name()),
            TermNode::Apply { target, args } => self.render_apply(*target, args),
            TermNode::Extension(ext) => ext.name().to_string(),
            TermNode::Constant(constant) => format!("Constant_{constant:?}"),
            TermNode::Hole(_) => format!("Hole"),
        }
    }

    ///Removes reference wrappers but preserves concrete generic applications such
    ///as `Option<int>`. The application node is the actual identity of a
    ///specialized type and must survive method/interface matching.
    pub fn dereference(self) -> HirViewer<'a, TermId> {
        let mut data = self.data;
        loop {
            match self.hir.types.storage.terms[data].node() {
                TermNode::Ref { target, .. } => data = *target,
                _ => break,
            }
        }
        self.new_with(data)
    }

    ///Retrieves the inner nominal type. This is, for any type T, which is being applied or referenced(such as, &int, Vector<i32>, Another<Thing>),
    ///it getts the base type for them, in case, the oen that contain a meaning and is being used as a base.
    /// For example `&int -> int`, `Vector<i32> -> Vector`, `Another<THing> -> Another`
    pub fn nominal(self) -> HirViewer<'a, TermId> {
        let mut data = self.data;
        loop {
            match self.hir.types.storage.terms[data].node() {
                TermNode::Ref { target, .. } => data = *target,
                TermNode::Apply { target, .. }
                    if self.new_with(*target).is_extension().is_none() =>
                {
                    data = *target;
                }
                _ => break,
            }
        }
        self.new_with(data)
    }

    ///Gets the concrete type by also unwrapping `&T`/`&mut T` on top of the
    ///named-type references, mirroring `concrete_type` of the `HirType` viewer.
    ///This intentionally preserves `Apply` nodes such as `Option<int>` so the
    ///specialized receiver identity stays intact for method/interface lookup.
    pub fn concrete_type(self) -> HirViewer<'a, TermId> {
        self.dereference()
    }

    pub fn is_application(self) -> Option<HirViewer<'a, (TermId, &'a Vec<TermId>)>> {
        if let TermNode::Apply { target, args } = self.raw().node() {
            Some(self.new_with((*target, args)))
        } else {
            None
        }
    }

    pub fn is_array(self) -> Option<(TermId, usize)> {
        if let TermNode::Apply { target, args } = self.raw().node()
            && let Some(true) = self
                .new_with(*target)
                .is_extension()
                .map(|ext| ext.dyn_eq(&ArrayTerm))
            && let (Some(elem), Some(len)) = (args.first(), args.get(1))
            && let Some(len) = self.constant_usize(*len)
        {
            return Some((*elem, len));
        }
        None
    }

    pub fn is_vector(self) -> Option<TermId> {
        if let TermNode::Apply { target, args } = self.raw().node()
            && let Some(true) = self
                .new_with(*target)
                .is_extension()
                .map(|ext| ext.dyn_eq(&VectorTerm))
        {
            return args.first().copied();
        }
        None
    }

    pub fn is_function(self) -> Option<(&'a [TermId], TermId)> {
        if let TermNode::Func { args, ret } = self.raw().node() {
            return Some((args, *ret));
        }
        None
    }

    pub fn is_struct(self) -> Option<HirViewer<'a, DedupPoolId<StructType>>> {
        if let TermNode::Data(DescriptorId::Struct(target)) = self.raw().node() {
            return Some(self.new_with(*target));
        }
        None
    }

    pub fn is_enum(self) -> Option<HirViewer<'a, DedupPoolId<EnumType>>> {
        if let TermNode::Data(DescriptorId::Enum(target)) = self.raw().node() {
            return Some(self.new_with(*target));
        }
        None
    }

    pub fn is_component(self) -> Option<HirViewer<'a, DedupPoolId<ComponentType>>> {
        if let TermNode::Data(DescriptorId::Component(target)) = self.raw().node() {
            return Some(self.new_with(*target));
        }
        None
    }

    pub fn is_tuple(self) -> Option<&'a [TermId]> {
        if let TermNode::Tuple { fields } = self.raw().node() {
            return Some(fields);
        }
        None
    }

    pub fn is_generic(self) -> Option<VarTerm> {
        if let TermNode::Var(var) = self.raw().node() {
            return Some(var.clone());
        }
        None
    }

    pub fn is_ref(self) -> bool {
        matches!(self.raw().node(), TermNode::Ref { .. })
    }

    pub fn is_imutable_ref(self) -> Option<TermId> {
        if let TermNode::Ref {
            mutable: false,
            target,
        } = self.raw().node()
        {
            return Some(*target);
        }
        None
    }

    pub fn is_mutable_ref(self) -> Option<TermId> {
        if let TermNode::Ref {
            mutable: true,
            target,
        } = self.raw().node()
        {
            return Some(*target);
        }
        None
    }

    pub fn is_extension(self) -> Option<Arc<dyn ExtensionNode>> {
        if let TermNode::Extension(ext) = self.raw().node() {
            return Some(ext.clone());
        }
        None
    }
    ///Renders the pretty name of this type. The pretty name is a name for the user to read, and should NOT be used to any internal naming
    pub fn pretty_name(self) -> String {
        let term = self.raw();
        match term.node() {
            TermNode::Primitive(pt) => pt.to_string(),
            TermNode::Var(var) => self.hir.get_name(var.name).into(),
            TermNode::Data(DescriptorId::Struct(s)) => {
                let name = self.hir.types.storage.get_struct_name(*s);
                self.hir.get_name(name).to_string()
            }
            TermNode::Data(DescriptorId::Enum(e)) => {
                let name = self.hir.types.storage.get_enum_name(*e);
                self.hir.get_name(name).to_string()
            }
            TermNode::Data(DescriptorId::Component(c)) => self.new_with(*c).name().to_string(),
            TermNode::Func { args, ret } => {
                let args = args
                    .iter()
                    .map(|arg| self.new_with(*arg).pretty_name())
                    .collect::<Vec<_>>()
                    .join(",");
                let ret = self.new_with(*ret).pretty_name();
                format!("func({args})->{ret}")
            }
            TermNode::Tuple { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| self.new_with(*field).pretty_name())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("({fields})")
            }
            TermNode::Ref {
                mutable: false,
                target,
            } => format!("&{}", self.new_with(*target).pretty_name()),
            TermNode::Ref {
                mutable: true,
                target,
            } => format!("&mut {}", self.new_with(*target).pretty_name()),
            TermNode::Apply { target, args } => self.render_apply(*target, args),
            TermNode::Extension(ext) => ext.name().to_string(),
            TermNode::Constant(_) | TermNode::Hole(_) => format!("{term:?}"),
        }
    }

    fn render_apply(self, target: TermId, args: &[TermId]) -> String {
        match self.new_with(target).is_extension() {
            Some(ext) if ext.dyn_eq(&ArrayTerm) => {
                let elem = args
                    .first()
                    .map(|elem| self.new_with(*elem).pretty_name())
                    .unwrap_or_default();
                let len = args
                    .get(1)
                    .and_then(|len| self.constant_usize(*len))
                    .unwrap_or_default();
                format!("[{len}]{elem}")
            }
            Some(ext) if ext.dyn_eq(&VectorTerm) => {
                let elem = args
                    .first()
                    .map(|elem| self.new_with(*elem).pretty_name())
                    .unwrap_or_default();
                format!("[]{elem}")
            }
            _ => {
                let base = self.new_with(target).pretty_name();
                let generics = args
                    .iter()
                    .map(|arg| self.new_with(*arg).pretty_name())
                    .collect::<Vec<_>>()
                    .join(",");
                if generics.is_empty() {
                    base
                } else {
                    format!("{base}<{generics}>")
                }
            }
        }
    }

    fn constant_usize(&self, id: TermId) -> Option<usize> {
        match self.hir.types.storage.terms[id].node() {
            TermNode::Constant(ConstantTerm::Usize(n)) => Some(*n),
            _ => None,
        }
    }
}

impl std::fmt::Display for HirViewer<'_, TermId> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty_name())
    }
}
