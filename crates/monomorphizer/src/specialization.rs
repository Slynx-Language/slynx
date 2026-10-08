//! The specialization request descriptor and the traits it is generic over.
//!
//! Monomorphizing every declaration kind follows the same recipe, so the
//! shared driver [`Monomorphizer::specialize`](crate::Monomorphizer::specialize)
//! is written once, generic over the kind being specialized. This module owns
//! the vocabulary it is generic over:
//!
//! - [`SpecializationDescriptor`] bundles one instantiation request — the typed
//!   template, the concrete type arguments, the use-site span, and the
//!   kind-specific callback that builds the copy — so `specialize` keeps a
//!   short, stable signature no matter how many inputs a declaration kind
//!   grows.
//! - [`NamedDeclaration`] is this pass's view of the name every declaration
//!   carries. The HIR's [`TypeDeclaration`] deliberately exposes only the
//!   type-level parts of a declaration (`hir_type`, `generics`), so the name
//!   accessor lives here.
//! - [`SpecializableDeclaration`] combines what specialization reads from
//!   every template with the narrowing of the erased id a cache hit yields
//!   (see [`SpecializableDeclaration::from_erased`]).
//!
//! Every trait here is implemented for the four declaration kinds the pass
//! specializes (function, object, component, enum) and lives nowhere else in
//! the compiler: they are local vocabulary for this crate, not new HIR
//! concepts.

use common::Span;
use slynx_hir::{
    DeclarationId, HirComponentDeclaration, HirDeclaration, HirEnumDeclaration,
    HirFunctionDeclaration, HirObjectDeclaration, TypeDeclaration,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
    term::TermId,
};

/// A single instantiation request handed to
/// [`Monomorphizer::specialize`](crate::Monomorphizer::specialize).
///
/// Every declaration kind triggers specialization the same way — "copy this
/// template with these type arguments" — but each supplies its own callback
/// that knows how to construct its kind of declaration. Bundling the inputs
/// into one descriptor keeps `specialize`'s signature at three parameters
/// regardless of how many inputs a kind needs, and gives every input a
/// documented home instead of a positional argument.
///
/// # Type parameters
///
/// - `K` — the declaration kind being specialized (`HirFunctionDeclaration`,
///   `HirObjectDeclaration`, `HirComponentDeclaration`, or
///   `HirEnumDeclaration`). The [`HirDeclaration`] bound brings the `Debug`
///   impl the typed [`DeclarationId`] field requires.
/// - `B` — the build callback; see [`SpecializationDescriptor::build`].
pub struct SpecializationDescriptor<K: HirDeclaration, B> {
    /// The generic template to copy, as a typed id. Its owner file is also the
    /// file the specialization is inserted into.
    pub template: DeclarationId<K>,
    /// The concrete type arguments of this instantiation, in parameter order.
    ///
    /// `specialize` rejects the request with `generic_arity_mismatch` when
    /// their count differs from the template's generic parameter count.
    pub args: Vec<TermId>,
    /// The span of the use site that requested the instantiation. Reported on
    /// arity mismatches and on cyclic instantiations.
    pub span: Span,
    /// Builds the specialization: inserts a fresh, empty-bodied declaration of
    /// kind `K` next to the template under the mangled name and returns its
    /// typed id.
    ///
    /// It is invoked as
    ///
    /// ```text
    /// (&mut Monomorphizer, &SlynxHir, &Substitution, SymbolPointer, &MonomorphizationKey)
    ///     -> Result<DeclarationId<K>>
    /// ```
    ///
    /// receiving the substitution for the type arguments, the interned mangled
    /// specialization name, and the cache key. The cache key is handed in so
    /// callbacks that must be visible to their own recursion (generic
    /// functions re-entering themselves through their body) can pre-populate
    /// the cache before the body is generated.
    pub build: B,
}

/// A declaration kind the monomorphizer knows how to specialize.
///
/// The supertraits cover everything the shared driver reads from every
/// template:
///
/// - [`TypeDeclaration`] supplies the generic arity that is checked against
///   the requested type arguments, and the explicit erasure to
///   [`AnyDeclarationId`] (`TypeDeclaration::as_any_id`) used to store ids in
///   the two heterogeneous collections of this pass (the cache and the
///   dead-code set).
/// - [`NamedDeclaration`] supplies the name used for diagnostics and for
///   mangling the specialization.
///
/// [`SpecializableDeclaration::from_erased`] is the mirror of `as_any_id`:
/// the monomorphization cache is the one deliberately heterogeneous store in
/// this pass — specializations of every declaration kind share a single map,
/// because a second typed map per kind would buy nothing — so a cache hit
/// yields an erased id that has to be narrowed back to `DeclarationId<Self>`
/// before the rest of the pass can stay typed.
pub trait SpecializableDeclaration: TypeDeclaration {
    /// Narrows an erased [`AnyDeclarationId`] back into this kind's typed
    /// [`DeclarationId`].
    ///
    /// Returns `None` when the id names a different declaration kind — a
    /// logic error, since the cache key embeds this kind's typed template id
    /// and a hit therefore can only ever name the same kind.
    fn from_erased(id: AnyDeclarationId) -> Option<DeclarationId<Self>>;
}

impl SpecializableDeclaration for HirFunctionDeclaration {
    fn from_erased(id: AnyDeclarationId) -> Option<DeclarationId<Self>> {
        let AnyLocalDeclarationId::Function(term) = id.term else {
            return None;
        };
        Some(DeclarationId::new(id.owner, term))
    }
}

impl SpecializableDeclaration for HirObjectDeclaration {
    fn from_erased(id: AnyDeclarationId) -> Option<DeclarationId<Self>> {
        let AnyLocalDeclarationId::Object(term) = id.term else {
            return None;
        };
        Some(DeclarationId::new(id.owner, term))
    }
}

impl SpecializableDeclaration for HirComponentDeclaration {
    fn from_erased(id: AnyDeclarationId) -> Option<DeclarationId<Self>> {
        let AnyLocalDeclarationId::Component(term) = id.term else {
            return None;
        };
        Some(DeclarationId::new(id.owner, term))
    }
}

impl SpecializableDeclaration for HirEnumDeclaration {
    fn from_erased(id: AnyDeclarationId) -> Option<DeclarationId<Self>> {
        let AnyLocalDeclarationId::Enum(term) = id.term else {
            return None;
        };
        Some(DeclarationId::new(id.owner, term))
    }
}
