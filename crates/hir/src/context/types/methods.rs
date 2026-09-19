use common::pool::DedupPoolId;
use dashmap::DashMap;

use crate::{DeclarationId, HirFunctionDeclaration, HirType, SymbolPointer};

#[derive(Debug)]
/// Methods attached to types.
///
/// Methods may be registered against **any** type id — structs, enums,
/// components, tuples, functions or built-ins — so that any value may carry
/// methods. The key is an arbitrary `DedupPoolId<HirType>` produced by the
/// type storage, so this table does not need to know what kind of type it is
/// keying methods off.
pub struct MethodTable {
    /// Maps a type id to the map of (method name → declaration) registered on it.
    methods: DashMap<
        DedupPoolId<HirType>,
        DashMap<SymbolPointer, DeclarationId<HirFunctionDeclaration>>,
    >,
    /// Maps (parent_type, method_name) -> return_type for external object methods.
    external_methods: DashMap<(DedupPoolId<HirType>, SymbolPointer), DedupPoolId<HirType>>,
}
impl Default for MethodTable {
    fn default() -> Self {
        Self {
            methods: DashMap::new(),
            external_methods: DashMap::new(),
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
        ty: DedupPoolId<HirType>,
        name: SymbolPointer,
        id: DeclarationId<HirFunctionDeclaration>,
    ) {
        self.methods.entry(ty).or_default().insert(name, id);
    }

    /// Looks up a single method registered on `ty`, if any. Works for methods
    /// attached to any type id.
    pub fn method_of(
        &self,
        ty: DedupPoolId<HirType>,
        name: SymbolPointer,
    ) -> Option<DeclarationId<HirFunctionDeclaration>> {
        self.methods.get(&ty)?.get(&name).map(|v| *v.value())
    }

    /// Whether any methods are registered for `ty`.
    pub fn has_methods(&self, ty: DedupPoolId<HirType>) -> bool {
        self.methods.contains_key(&ty)
    }

    /// Register an external method's return type without creating a declaration entry.
    pub fn register_external_method(
        &self,
        parent_ty: DedupPoolId<HirType>,
        name: SymbolPointer,
        return_type: DedupPoolId<HirType>,
    ) {
        self.external_methods.insert((parent_ty, name), return_type);
    }

    /// Returns the return type of an external method on `parent_ty` with the given `name`.
    pub fn get_method_return_type(
        &self,
        parent_ty: &DedupPoolId<HirType>,
        name: SymbolPointer,
    ) -> Option<DedupPoolId<HirType>> {
        self.external_methods
            .get(&(*parent_ty, name))
            .map(|ret| *ret.value())
    }

    ///Returns the methods registered on the given `ty`, if any.
    pub fn get_methods_of(
        &self,
        ty: DedupPoolId<HirType>,
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
}
