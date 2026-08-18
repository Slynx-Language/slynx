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

use common::pool::DedupPoolId;
use slynx_hir::{HirType, Result, SlynxHir, SymbolPointer, id::AnyDeclarationId};
use smallvec::SmallVec;

/// Maps a generic parameter index to the concrete type it should be
/// substituted with.
#[derive(Debug, Clone, Default)]
pub(crate) struct Substitution(HashMap<u8, DedupPoolId<HirType>>);

/// The key of a monomorphization: the generic template declaration together
/// with the concrete type arguments supplied at a use site.
pub(crate) type MonomorphizationKey = (AnyDeclarationId, SmallVec<[DedupPoolId<HirType>; 2]>);

impl Substitution {
    ///Builds the substitution from a template's type-parameter list and the
    ///concrete type arguments supplied at a use site.
    pub(crate) fn new(args: &[DedupPoolId<HirType>]) -> Substitution {
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

    fn get(&self, index: &u8) -> Option<&DedupPoolId<HirType>> {
        self.0.get(index)
    }

    fn contains_key(&self, index: &u8) -> bool {
        self.0.contains_key(index)
    }
}

///Computes the mangled name of a specialization: the template name followed by
///one `_<name>_<hash>` segment per concrete type argument, where the hash is
///computed structurally over the `HirType` value. The name stays unique per
///type-argument list while remaining human-readable.
pub(crate) fn mangle_name(
    hir: &SlynxHir,
    name: SymbolPointer,
    args: &[DedupPoolId<HirType>],
) -> String {
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
pub(crate) fn substitute_type(
    hir: &SlynxHir,
    ty: DedupPoolId<HirType>,
    subst: &Substitution,
) -> Result<DedupPoolId<HirType>> {
    match hir.view(ty).raw() {
        HirType::GenericParam { index, .. } => Ok(subst.get(index).copied().unwrap_or(ty)),
        HirType::Array(inner, len) => {
            Ok(hir.create_type(HirType::Array(substitute_type(hir, *inner, subst)?, *len)))
        }
        HirType::Vector(inner) => {
            Ok(hir.create_type(HirType::Vector(substitute_type(hir, *inner, subst)?)))
        }
        HirType::Function(function) => {
            let function_view = hir.view(*function);
            let args = function_view
                .arguments()
                .iter()
                .map(|arg| substitute_type(hir, *arg, subst))
                .collect::<Result<Vec<_>>>()?;
            let ret = substitute_type(hir, function_view.return_type(), subst)?;
            Ok(hir.create_function_type(args, ret))
        }
        HirType::Tuple(tuple) => {
            let tuple_view = hir.view(*tuple);
            let fields = tuple_view
                .fields()
                .iter()
                .map(|field| substitute_type(hir, *field, subst))
                .collect::<Result<Vec<_>>>()?;
            Ok(hir.create_tuple_type(fields))
        }
        HirType::Reference { rf, generics } => {
            let new_rf = substitute_type(hir, *rf, subst)?;
            let mut new_generics = *generics;
            for slot in &mut new_generics {
                if !slot.is_null()
                    && let HirType::GenericParam { index, .. } = hir.view(*slot).raw()
                    && subst.contains_key(index)
                {
                    *slot = substitute_type(hir, *slot, subst)?;
                }
            }
            Ok(hir.create_type(HirType::Reference {
                rf: new_rf,
                generics: new_generics,
            }))
        }
        HirType::Nullable(inner) => {
            let new_inner = substitute_type(hir, *inner, subst)?;
            Ok(hir.create_type(HirType::Nullable(new_inner)))
        }
        other => Ok(hir.create_type(other.clone())),
    }
}

///Returns `true` if `ty` is a [`HirType::Reference`] carrying concrete type
///arguments (no unresolved generic parameter anywhere), and therefore a
///candidate for specialization by the struct/component modules.
pub(crate) fn is_resolvable_reference(hir: &SlynxHir, ty: DedupPoolId<HirType>) -> bool {
    let ty_view = hir.view(ty);
    let HirType::Reference { generics, .. } = ty_view.raw() else {
        return false;
    };
    let concrete: Vec<DedupPoolId<HirType>> = generics
        .iter()
        .filter(|slot| !slot.is_null())
        .copied()
        .collect();
    !concrete.is_empty()
        && concrete
            .iter()
            .all(|slot| !contains_generic_param(hir, *slot))
}

///Returns `true` if `ty` contains an unresolved [`HirType::GenericParam`]
///anywhere in its structure.
pub(crate) fn contains_generic_param(hir: &SlynxHir, ty: DedupPoolId<HirType>) -> bool {
    match hir.view(ty).raw() {
        HirType::GenericParam { .. } => true,
        HirType::Array(inner, _) | HirType::Vector(inner) | HirType::Nullable(inner) => {
            contains_generic_param(hir, *inner)
        }
        HirType::Function(function) => {
            let view = hir.view(*function);
            view.arguments()
                .iter()
                .any(|arg| contains_generic_param(hir, *arg))
                || contains_generic_param(hir, view.return_type())
        }
        HirType::Tuple(tuple) => hir
            .view(*tuple)
            .fields()
            .iter()
            .any(|field| contains_generic_param(hir, *field)),
        HirType::Reference { rf, generics } => {
            contains_generic_param(hir, *rf)
                || generics
                    .iter()
                    .any(|slot| !slot.is_null() && contains_generic_param(hir, *slot))
        }
        _ => false,
    }
}

///Returns `true` if `ty` contains a [`HirType::Reference`] with concrete type
///arguments that targets a generic struct or component — i.e. a type that
///`resolve_expression_type` would need to specialize.
pub(crate) fn contains_resolvable_reference(hir: &SlynxHir, ty: DedupPoolId<HirType>) -> bool {
    match hir.view(ty).raw() {
        HirType::Reference { rf, generics } => {
            let concrete: Vec<DedupPoolId<HirType>> = generics
                .iter()
                .filter(|slot| !slot.is_null())
                .copied()
                .collect();
            if !concrete.is_empty()
                && concrete
                    .iter()
                    .all(|slot| !contains_generic_param(hir, *slot))
            {
                let ty_view = hir.view(*rf);
                let deref = ty_view.dereference();
                if deref.is_struct().is_some() || deref.is_component().is_some() {
                    return true;
                }
            }
            contains_resolvable_reference(hir, *rf)
                || generics
                    .iter()
                    .any(|slot| !slot.is_null() && contains_resolvable_reference(hir, *slot))
        }
        HirType::Array(inner, _) | HirType::Vector(inner) => {
            contains_resolvable_reference(hir, *inner)
        }
        HirType::Function(function) => {
            let view = hir.view(*function);
            view.arguments()
                .iter()
                .any(|arg| contains_resolvable_reference(hir, *arg))
                || contains_resolvable_reference(hir, view.return_type())
        }
        HirType::Tuple(tuple) => hir
            .view(*tuple)
            .fields()
            .iter()
            .any(|field| contains_resolvable_reference(hir, *field)),
        HirType::Component(component) => {
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
