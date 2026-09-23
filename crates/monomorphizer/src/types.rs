//! Type machinery shared by every monomorphization module.
//!
//! [`Substitution`] maps generic-parameter indices to the concrete types they
//! should be replaced with. [`substitute_type`] walks an [`HirType`] and
//! performs that replacement. [`mangle_name`] derives a stable, unique name for
//! a specialization from the template name and its type arguments.

use std::{
    collections::HashMap,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use slynx_hir::{
    DescriptorId, EnumVariantType, Result, SlynxHir, SymbolPointer,
    id::AnyDeclarationId,
    term::{Term, TermId, TermNode, VarTerm},
};
use smallvec::SmallVec;

/// Maps a generic parameter index to the concrete type it should be
/// substituted with.
#[derive(Debug, Clone, Default)]
pub(crate) struct Substitution(HashMap<u8, TermId>);

/// The key of a monomorphization: the generic template declaration together
/// with the concrete type arguments supplied at a use site.
pub(crate) type MonomorphizationKey = (AnyDeclarationId, SmallVec<[TermId; 2]>);

impl Substitution {
    ///Builds the substitution from a template's type-parameter list and the
    ///concrete type arguments supplied at a use site.
    pub(crate) fn new(args: &[TermId]) -> Substitution {
        let mut subst = HashMap::new();
        for (index, arg) in args.iter().enumerate() {
            subst.insert(index as u8, *arg);
        }
        Substitution(subst)
    }

    ///An empty substitution, i.e. the identity mapping.
    pub(crate) fn empty() -> Substitution {
        Substitution(HashMap::new())
    }

    fn get(&self, index: &u8) -> Option<TermId> {
        self.0.get(index).cloned()
    }
}

///Computes the mangled name of a specialization: the template name followed by
///one `_<name>_<hash>` segment per concrete type argument, where the hash is
///computed structurally over the `HirType` value. The name stays unique per
///type-argument list while remaining human-readable.
pub(crate) fn mangle_name(hir: &SlynxHir, name: SymbolPointer, args: &[TermId]) -> String {
    let base = hir.get_name(name);
    let mut out = String::with_capacity(base.len() + args.len() * 20);
    out.push_str(base);
    for arg in args {
        let mut hasher = DefaultHasher::new();
        hir.view(*arg).raw().hash(&mut hasher);
        out.push('_');
        out.push_str(&hir.view(*arg).name());
        out.push_str(&format!("_{:04x}", hasher.finish() & 0xffff));
    }
    out
}

///Substitutes every generic parameter inside `ty` with its concrete type.
pub(crate) fn substitute_type(hir: &SlynxHir, ty: TermId, subst: &Substitution) -> Result<TermId> {
    match hir.view(ty).raw().node() {
        TermNode::Func { args, ret } => {
            let args = args
                .iter()
                .map(|arg| substitute_type(hir, *arg, subst))
                .collect::<Result<Vec<_>>>()?;
            let ret = substitute_type(hir, *ret, subst)?;
            Ok(hir
                .types
                .create_term(Term::new_type(TermNode::Func { args, ret })))
        }
        TermNode::Var(VarTerm { index, .. }) => Ok(subst.get(index).unwrap_or(ty)),

        TermNode::Tuple { fields } => {
            let fields = fields
                .iter()
                .map(|field| substitute_type(hir, *field, subst))
                .collect::<Result<Vec<_>>>()?;
            Ok(hir
                .types
                .create_term(Term::new_type(TermNode::Tuple { fields })))
        }
        TermNode::Apply { target, args } => {
            let new_rf = substitute_type(hir, *target, subst)?;
            let new_generics = args
                .iter()
                .map(|arg| substitute_type(hir, *arg, subst))
                .collect::<Result<Vec<_>>>()?;
            Ok(hir.types.create_term(Term::new_type(TermNode::Apply {
                target: new_rf,
                args: new_generics,
            })))
        }

        TermNode::Data(data) if let DescriptorId::Enum(data) = data => {
            let enum_view = hir.view(*data);
            let variants = enum_view
                .variants()
                .iter()
                .map(|variant| {
                    let payload = variant
                        .payload
                        .iter()
                        .map(|ty| substitute_type(hir, *ty, subst))
                        .collect::<Result<Vec<_>>>()?;
                    Ok(EnumVariantType {
                        name: variant.name,
                        discriminant: variant.discriminant,
                        payload,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(hir.types.create_enum_type(enum_view.name(), variants))
        }
        _ => Ok(ty),
    }
}

///Returns `true` if `ty` is a type application carrying concrete type arguments
///(no unresolved generic parameter anywhere) that targets a generic struct,
///component, or enum — and therefore a candidate for specialization by the
///struct/component modules.
pub(crate) fn is_resolvable_reference(hir: &SlynxHir, ty: TermId) -> bool {
    let ty_view = hir.view(ty);
    let TermNode::Apply { args, .. } = ty_view.raw().node() else {
        return false;
    };
    let concrete: Vec<TermId> = args
        .iter()
        .filter(|slot| !slot.is_null())
        .copied()
        .collect();
    if concrete.is_empty()
        || !concrete
            .iter()
            .all(|slot| !contains_generic_param(hir, *slot))
    {
        return false;
    }
    // The application must specialize a generic struct, component, or enum.
    // Collection applications (`Vector<T>`, `Array<T, N>`) are `Apply` terms over
    // built-in extensions and must be handled by the generic `Apply` arm instead:
    // the specialization paths only accept structs, components, and enums, and
    // would otherwise hit `unreachable!` in `resolve_expression_type`.
    let deref = ty_view.dereference();
    deref.is_struct().is_some() || deref.is_component().is_some() || deref.is_enum().is_some()
}

///Returns `true` if `ty` contains an unresolved [`HirType::GenericParam`]
///anywhere in its structure.
pub(crate) fn contains_generic_param(hir: &SlynxHir, ty: TermId) -> bool {
    match hir.view(ty).raw().node() {
        TermNode::Var(_) => true,
        TermNode::Extension(ext) => ext
            .children()
            .iter()
            .any(|child| contains_generic_param(hir, *child)),
        TermNode::Func { args, ret } => {
            args.iter().any(|arg| contains_generic_param(hir, *arg))
                || contains_generic_param(hir, *ret)
        }
        TermNode::Tuple { fields } => fields
            .iter()
            .any(|field| contains_generic_param(hir, *field)),
        TermNode::Apply { target, args } => {
            contains_generic_param(hir, *target)
                || args
                    .iter()
                    .any(|slot| !slot.is_null() && contains_generic_param(hir, *slot))
        }
        TermNode::Data(descriptor) if let DescriptorId::Enum(enum_id) = descriptor => {
            hir.view(*enum_id).variants().iter().any(|variant| {
                variant
                    .payload
                    .iter()
                    .any(|payload| contains_generic_param(hir, *payload))
            })
        }

        _ => false,
    }
}

///Returns `true` if `ty` contains a [`HirType::Reference`] with concrete type
///arguments that targets a generic struct or component — i.e. a type that
///`resolve_expression_type` would need to specialize.
pub(crate) fn contains_resolvable_reference(hir: &SlynxHir, ty: TermId) -> bool {
    match hir.view(ty).raw().node() {
        TermNode::Apply { args, target } => {
            let concrete: Vec<TermId> = args
                .iter()
                .filter(|slot| !slot.is_null())
                .copied()
                .collect();
            if !concrete.is_empty()
                && concrete
                    .iter()
                    .all(|slot| !contains_generic_param(hir, *slot))
            {
                let ty_view = hir.view(*target);
                let deref = ty_view.dereference();
                if deref.is_struct().is_some()
                    || deref.is_component().is_some()
                    || deref.is_enum().is_some()
                {
                    return true;
                }
            }
            contains_resolvable_reference(hir, *target)
                || args
                    .iter()
                    .any(|slot| !slot.is_null() && contains_resolvable_reference(hir, *slot))
        }
        TermNode::Extension(ext) => ext
            .children()
            .iter()
            .any(|child| contains_resolvable_reference(hir, *child)),

        TermNode::Func { args, ret } => {
            args.iter()
                .any(|arg| contains_resolvable_reference(hir, *arg))
                || contains_resolvable_reference(hir, *ret)
        }
        TermNode::Tuple { fields } => fields
            .iter()
            .any(|field| contains_resolvable_reference(hir, *field)),
        TermNode::Data(component) if let DescriptorId::Component(component) = component => {
            let view = hir.view(*component);
            view.props()
                .iter()
                .any(|prop| contains_resolvable_reference(hir, *prop))
                || view.children().iter().any(|child| {
                    let child = hir.view(*child);
                    child
                        .props()
                        .iter()
                        .any(|prop| contains_resolvable_reference(hir, *prop))
                })
        }
        _ => false,
    }
}
