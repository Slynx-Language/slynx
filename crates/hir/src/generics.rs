//! Shared generic-related infrastructure for HIR declarations.
//!
//! Generic support for a declaration conceptually decomposes into three steps
//! that every generic declaration shares:
//!
//!   1. **Declaring** named type parameters — a [`Vec`]`<SymbolPointer>` stored
//!      on the declaration itself (the `generics` field of each declaration
//!      model). The position of a parameter in this vector is its generic
//!      index, and is what [`HirType::GenericParam`] refer to.
//!
//!   2. **Building a mapping** from those parameters (by index) to concrete
//!      type arguments — [`GenericTypeArguments`]. Holes are filled either from
//!      explicit type arguments at the use site or inferred from the argument
//!      expressions, so the mapping is accumulated incrementally.
//!
//!   3. **Resolving / substituting** types against that mapping —
//!      [`substitute_types`].
//!
//! By centralizing steps 2 and 3, a new generic declaration only needs to build
//! its own parameters/arguments; the mapping, inference, and substitution
//! machinery is shared.

use common::{Spanned, pool::DedupPoolId};
use slynx_parser::{Type, TypeContext};

use crate::{HirType, Result, SlynxHir, builders::HirNode};

/// A mapping from a declaration's type-parameter index to a concrete type
/// argument.
///
/// Slots are indexed by the parameter position declared on the generic
/// declaration (matching the `index` of [`HirType::GenericParam`]). A null slot
/// means "not resolved yet". This is the shared buffer generic declarations use
/// to accumulate the concrete arguments of a generic reference — either from
/// explicit type arguments at the use site or inferred from argument
/// expressions — before resolving or substituting types against it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GenericTypeArguments {
    slots: Vec<DedupPoolId<HirType>>,
}

impl GenericTypeArguments {
    /// Creates an empty mapping with no declared parameters.
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    /// Creates a mapping seeded from a pre-resolved list of explicit type
    /// arguments. `explicit` is indexed by generic-parameter position.
    pub fn from_explicit(explicit: Vec<DedupPoolId<HirType>>) -> Self {
        Self { slots: explicit }
    }

    /// Number of type parameters this mapping currently tracks.
    pub fn arity(&self) -> usize {
        self.slots.len()
    }

    /// Declares parameter slots `0..arity` as unresolved, preserving any that
    /// are already present. Used when the full arity of a declaration is known
    /// up front (e.g. from a variant's payload) rather than discovered by
    /// inference.
    pub fn reserve(&mut self, arity: usize) {
        self.slots.resize(arity, DedupPoolId::new_null());
    }

    /// The resolved argument for parameter `index`, if any.
    pub fn get(&self, index: usize) -> Option<DedupPoolId<HirType>> {
        self.slots
            .get(index)
            .copied()
            .filter(|ty| !ty.is_null())
    }

    /// Whether the parameter at `index` has been resolved.
    pub fn is_resolved(&self, index: usize) -> bool {
        self.get(index).is_some()
    }

    /// Sets the argument for parameter `index`, growing the mapping as needed.
    pub fn set(&mut self, index: usize, ty: DedupPoolId<HirType>) {
        if self.slots.len() <= index {
            self.slots.resize(index + 1, DedupPoolId::new_null());
        }
        self.slots[index] = ty;
    }

    /// Sets the argument for `index` only if it is not already resolved.
    /// Returns `true` if the slot was empty and got filled.
    ///
    /// This is the "explicit arguments win; otherwise infer" rule used when
    /// each parameter may receive an inferred value at most once.
    pub fn try_set(&mut self, index: usize, ty: DedupPoolId<HirType>) -> bool {
        if self.is_resolved(index) {
            false
        } else {
            self.set(index, ty);
            true
        }
    }

    /// Back-fills any still-unresolved, already-declared slots from a list of
    /// explicit type arguments, indexed by parameter position. Explicit
    /// arguments are only applied to parameters that were declared (see
    /// [`Self::reserve`]) and were not already resolved via inference.
    pub fn merge_explicit(&mut self, explicit: &[DedupPoolId<HirType>]) {
        for (index, ty) in explicit.iter().enumerate() {
            if index < self.arity() && !self.is_resolved(index) {
                self.set(index, *ty);
            }
        }
    }

    /// Substitutes every generic parameter in `ty` with its argument from this
    /// mapping. Parameters with no resolved argument are left as-is.
    pub fn substitute(&self, hir: &SlynxHir, ty: DedupPoolId<HirType>) -> DedupPoolId<HirType> {
        substitute_types(hir, &self.slots, ty)
    }

    /// Builds a generic [`HirType::Reference`] to `rf` carrying the resolved
    /// arguments, preserving unresolved (null) slots in the same positions they
    /// occupy in the mapping.
    pub fn finish_ref(&self, hir: &SlynxHir, rf: DedupPoolId<HirType>) -> DedupPoolId<HirType> {
        hir.create_type(HirType::new_generic_ref(rf, self.slots.clone()))
    }

    /// Consumes the mapping into its raw slot list, indexed by parameter
    /// position (unresolved slots remain [`DedupPoolId::new_null`]).
    pub fn into_vec(self) -> Vec<DedupPoolId<HirType>> {
        self.slots
    }
}

/// Replaces every [`HirType::GenericParam`] inside `ty` with the matching type
/// argument from `generics` (indexed by parameter position), recursing through
/// container types.
///
/// This is the substitution half of the generic pipeline: given a concrete
/// generic reference (e.g. `Container<int>`), it concretizes a type that still
/// mentions the reference's type parameters (e.g. accessing `Container<T>`'s
/// `T`-typed field yields `int` instead of a leftover `GenericParam`).
pub fn substitute_types(
    hir: &SlynxHir,
    generics: &[DedupPoolId<HirType>],
    ty: DedupPoolId<HirType>,
) -> DedupPoolId<HirType> {
    match hir.view(ty).raw() {
        HirType::GenericParam { index, .. } => {
            generics.get(*index as usize).copied().unwrap_or(ty)
        }
        HirType::Array(inner, len) => {
            let inner = substitute_types(hir, generics, *inner);
            hir.create_type(HirType::Array(inner, *len))
        }
        HirType::Vector(inner) => {
            let inner = substitute_types(hir, generics, *inner);
            hir.create_type(HirType::Vector(inner))
        }
        HirType::Nullable(inner) => {
            let inner = substitute_types(hir, generics, *inner);
            hir.create_type(HirType::Nullable(inner))
        }
        HirType::Tuple(tuple) => {
            let fields = hir
                .view(*tuple)
                .fields()
                .iter()
                .map(|field| substitute_types(hir, generics, *field))
                .collect::<Vec<_>>();
            hir.create_tuple_type(fields)
        }
        HirType::Function(function) => {
            let function_view = hir.view(*function);
            let args = function_view
                .arguments()
                .iter()
                .map(|arg| substitute_types(hir, generics, *arg))
                .collect::<Vec<_>>();
            let ret = substitute_types(hir, generics, function_view.return_type());
            hir.create_function_type(args, ret)
        }
        HirType::Reference {
            rf,
            generics: inner_generics,
        } => {
            let rf = substitute_types(hir, generics, *rf);
            let mut new_generics = *inner_generics;
            for slot in &mut new_generics {
                if !slot.is_null() {
                    *slot = substitute_types(hir, generics, *slot);
                }
            }
            hir.create_type(HirType::Reference {
                rf,
                generics: new_generics,
            })
        }
        _ => ty,
    }
}

/// Computes the generic arity (the highest generic-parameter index referenced
/// by any of `types`, plus one) implied by a set of payload types.
///
/// Used to size a generic reference when a declaration does not expose its
/// declared parameter count directly.
pub fn implied_arity(types: &[DedupPoolId<HirType>], hir: &SlynxHir) -> usize {
    types
        .iter()
        .filter_map(|ty| match hir.view(*ty).raw() {
            HirType::GenericParam { index, .. } => Some(*index as usize + 1),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

impl HirNode<'_> {
    /// Resolves the explicit generic type arguments of a call like
    /// `compare<int>(a, b)` into their HIR type ids. Types that are generic
    /// parameters of the enclosing declaration (e.g. `identity<T>(x)`) resolve
    /// to [`HirType::GenericParam`] ids, which monomorphization later
    /// substitutes with concrete types.
    pub(crate) fn resolve_call_generics(
        &self,
        generics: &[Spanned<DedupPoolId<Type>>],
        context: &TypeContext,
    ) -> Result<Vec<DedupPoolId<HirType>>> {
        generics
            .iter()
            .map(|ty| self.find_type(*ty, context).map(|(_, ty)| ty))
            .collect()
    }
}
