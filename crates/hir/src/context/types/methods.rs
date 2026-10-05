use common::pool::DedupPoolId;
use dashmap::{DashMap, mapref::one::Ref};

use crate::{
    DeclarationId, HirExtendDeclaration, HirFunctionDeclaration, InterfaceType, SymbolPointer,
    term::TermId,
};

#[derive(Debug, Clone, Copy)]
/// A method declared by an interface, materialized as a bodyless
/// [`HirFunctionDeclaration`] whose signature is keyed by `Self = Var(0)`.
///
/// A call on a bounded generic parameter (`func f<T>(x: T) where T: I` calling
/// `x.m()`) has no concrete target while the HIR is being built, so it resolves
/// to the interface's signature declaration and is discharged later, once the
/// receiver's concrete type is known.
pub struct InterfaceMethodSignature {
    ///The interface that declares the method.
    pub interface: DedupPoolId<InterfaceType>,
    ///The name the method is declared under.
    pub name: SymbolPointer,
    ///The bodyless declaration the deferred call resolves to.
    pub declaration: DeclarationId<HirFunctionDeclaration>,
}

#[derive(Debug)]
/// Methods attached to types.
///
/// Methods may be registered against **any** type id — structs, enums,
/// components, tuples, functions or built-ins — so that any value may carry
/// methods. The key is an arbitrary `TermId` produced by the
/// type storage, so this table does not need to know what kind of type it is
/// keying methods off.
pub struct MethodTable {
    /// Maps a type id to the map of (method name → declaration) registered on it.
    methods: DashMap<TermId, DashMap<SymbolPointer, DeclarationId<HirFunctionDeclaration>>>,
    /// Maps (parent_type, method_name) -> return_type for external object methods.
    external_methods: DashMap<(TermId, SymbolPointer), TermId>,
    ///Maps a type to an interface type. This occurs when a type is extended with an interface. Such as `extend int: ToString {}` this'd be `int` → `ToString`.
    ///This maps to a `Vec` of `HirExtendDeclaration` ids, as a type may have multiple extensions
    pub(crate) extensions: DashMap<TermId, Vec<DeclarationId<HirExtendDeclaration>>>,
    ///The signature declarations of every materialized interface, keyed by the interface that declares them.
    interface_methods: DashMap<DedupPoolId<InterfaceType>, Vec<InterfaceMethodSignature>>,
    ///Reverse index from a signature declaration back to the interface method it stands for.
    interface_signatures: DashMap<DeclarationId<HirFunctionDeclaration>, InterfaceMethodSignature>,
}
impl Default for MethodTable {
    fn default() -> Self {
        Self {
            methods: DashMap::new(),
            external_methods: DashMap::new(),
            extensions: DashMap::new(),
            interface_methods: DashMap::new(),
            interface_signatures: DashMap::new(),
        }
    }
}
impl MethodTable {
    /// Creates an empty method table.
    pub fn new() -> Self {
        Self::default()
    }

    ///Registers a method for the given `ty` on the current declaration context with the given `name` that points to the given `id`. It should be asserted by the HIR to be a function ID
    pub fn create_method(
        &self,
        ty: TermId,
        name: SymbolPointer,
        id: DeclarationId<HirFunctionDeclaration>,
    ) {
        self.methods.entry(ty).or_default().insert(name, id);
    }

    pub fn get_extensions_of<'a>(
        &'a self,
        ty: TermId,
    ) -> Option<Ref<'a, TermId, Vec<DeclarationId<HirExtendDeclaration>>>> {
        if let Some(extension) = self.extensions.get(&ty) {
            Some(extension)
        } else {
            None
        }
    }

    pub fn create_extension(&self, ty: TermId, extension: DeclarationId<HirExtendDeclaration>) {
        self.extensions.entry(ty).or_default().push(extension);
    }

    /// Looks up a single method registered on `ty`, if any. Works for methods
    /// attached to any type id.
    pub fn method_of(
        &self,
        ty: TermId,
        name: SymbolPointer,
    ) -> Option<DeclarationId<HirFunctionDeclaration>> {
        self.methods.get(&ty)?.get(&name).map(|v| *v.value())
    }

    /// Whether any methods are registered for `ty`.
    pub fn has_methods(&self, ty: TermId) -> bool {
        self.methods.contains_key(&ty)
    }

    /// Register an external method's return type without creating a declaration entry.
    pub fn register_external_method(
        &self,
        parent_ty: TermId,
        name: SymbolPointer,
        return_type: TermId,
    ) {
        self.external_methods.insert((parent_ty, name), return_type);
    }

    /// Returns the return type of an external method on `parent_ty` with the given `name`.
    pub fn get_method_return_type(
        &self,
        parent_ty: &TermId,
        name: SymbolPointer,
    ) -> Option<TermId> {
        self.external_methods
            .get(&(*parent_ty, name))
            .map(|ret| *ret.value())
    }

    ///Returns the methods registered on the given `ty`, if any.
    pub fn get_methods_of(
        &self,
        ty: TermId,
    ) -> Vec<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)> {
        if let Some(methods_map) = self.methods.get(&ty) {
            let mut out = Vec::with_capacity(methods_map.len());
            for entry in methods_map.iter() {
                let (key, value) = entry.pair();
                out.push((*key, *value));
            }
            out
        } else {
            Vec::new()
        }
    }

    ///Registers the signature of a method declared by the given `interface`.
    pub fn create_interface_method(&self, signature: InterfaceMethodSignature) {
        self.interface_methods
            .entry(signature.interface)
            .or_default()
            .push(signature);
        self.interface_signatures
            .insert(signature.declaration, signature);
    }

    ///Returns the signature declaration of the method `name` declared by the
    ///given `interface`, if that interface has been materialized.
    pub fn interface_method_of(
        &self,
        interface: DedupPoolId<InterfaceType>,
        name: SymbolPointer,
    ) -> Option<DeclarationId<HirFunctionDeclaration>> {
        self.interface_methods
            .get(&interface)?
            .iter()
            .find(|signature| signature.name == name)
            .map(|signature| signature.declaration)
    }

    ///Returns the interface method the given `declaration` is the signature of,
    ///or `None` when it is an ordinary function declaration.
    pub fn interface_signature(
        &self,
        declaration: DeclarationId<HirFunctionDeclaration>,
    ) -> Option<InterfaceMethodSignature> {
        self.interface_signatures
            .get(&declaration)
            .map(|signature| *signature.value())
    }
}
