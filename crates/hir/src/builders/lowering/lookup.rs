use common::pool::PoolId;
use dashmap::DashMap;
use module_loader::{ASTType, FileId, Modules};
use slynx_parser::{
    ASTDeclaration, EnumDeclaration, ExtendDeclaration, FuncDeclaration, InterfaceDeclaration,
    StaticDeclaration,
};

use crate::{Owned, SymbolPointer};

type EnumVariant = (Owned<PoolId<EnumDeclaration>>, usize);

pub struct ASTLookup<'a> {
    modules: &'a Modules<'a>,
    type_cache: DashMap<Owned<SymbolPointer>, Option<ASTType>>,
    function_cache: DashMap<Owned<SymbolPointer>, Option<Owned<PoolId<FuncDeclaration>>>>,
    interface_cache: DashMap<Owned<SymbolPointer>, Option<Owned<PoolId<InterfaceDeclaration>>>>,
    enum_cache: DashMap<Owned<SymbolPointer>, Option<EnumVariant>>,
    static_cache: DashMap<Owned<SymbolPointer>, Option<Owned<PoolId<StaticDeclaration>>>>,
    extension_method_cache:
        DashMap<FindExtensionsWithMethodDescriptor, Vec<Owned<PoolId<ExtendDeclaration>>>>,
    reachable_modules_cache: DashMap<FileId, Vec<FileId>>,
}

///A descriptor for finding extension declarations that provide a method.
#[derive(PartialEq, Eq, Hash)]
pub struct FindExtensionsWithMethodDescriptor {
    pub requester: FileId,
    pub method_name: SymbolPointer,
}

impl<'a> ASTLookup<'a> {
    pub fn new(modules: &'a Modules<'a>) -> Self {
        Self {
            modules,
            type_cache: DashMap::new(),
            function_cache: DashMap::new(),
            interface_cache: DashMap::new(),
            enum_cache: DashMap::new(),
            static_cache: DashMap::new(),
            extension_method_cache: DashMap::new(),
            reachable_modules_cache: DashMap::new(),
        }
    }

    pub fn find_type(&self, requester: FileId, name: SymbolPointer) -> Option<ASTType> {
        let cache_key = Owned {
            owner: requester,
            term: name,
        };
        if let Some(cached) = self.type_cache.get(&cache_key) {
            return *cached;
        }
        let result = self.modules.find_type(requester, name);
        self.type_cache.insert(cache_key, result);
        result
    }

    pub fn find_function(
        &self,
        name: SymbolPointer,
        requester: FileId,
    ) -> Option<Owned<PoolId<slynx_parser::FuncDeclaration>>> {
        let cache_key = Owned {
            owner: requester,
            term: name,
        };
        if let Some(cached) = self.function_cache.get(&cache_key) {
            return *cached;
        }
        let out = self
            .modules
            .find_declaration(name, requester)
            .map(|(owner, term)| Owned { owner, term });

        self.function_cache.insert(cache_key, out);
        out
    }

    pub fn find_interface(
        &self,
        name: SymbolPointer,
        requester: FileId,
    ) -> Option<Owned<PoolId<slynx_parser::InterfaceDeclaration>>> {
        let cache_key = Owned {
            owner: requester,
            term: name,
        };
        if let Some(cached) = self.interface_cache.get(&cache_key) {
            return *cached;
        }
        let out = self
            .modules
            .find_declaration(name, requester)
            .map(|(owner, term)| Owned { owner, term });

        self.interface_cache.insert(cache_key, out);
        out
    }

    pub fn find_enum_variant(
        &self,
        name: SymbolPointer,
        requester: FileId,
    ) -> Option<(Owned<PoolId<slynx_parser::EnumDeclaration>>, usize)> {
        let cache_key = Owned {
            owner: requester,
            term: name,
        };
        if let Some(cached) = self.enum_cache.get(&cache_key) {
            return *cached;
        }
        let out = self
            .modules
            .find_enum_variant(name, requester)
            .map(|(owner, term, id)| (Owned { owner, term }, id));

        self.enum_cache.insert(cache_key, out);
        out
    }

    pub fn find_static(
        &self,
        name: SymbolPointer,
        requester: FileId,
    ) -> Option<Owned<PoolId<StaticDeclaration>>> {
        let cache_key = Owned {
            owner: requester,
            term: name,
        };
        if let Some(cached) = self.static_cache.get(&cache_key) {
            return *cached;
        }
        let result = self
            .modules
            .find_declaration(name, requester)
            .map(|(owner, term)| Owned { owner, term });
        self.static_cache.insert(cache_key, result);
        result
    }

    /// Finds every reachable extension that declares a method with this name.
    /// Results are AST declaration IDs only; lowering and implementation
    /// materialization are intentionally left to their respective phases.
    pub fn find_extensions_with_method(
        &self,
        descriptor: FindExtensionsWithMethodDescriptor,
    ) -> Vec<Owned<PoolId<ExtendDeclaration>>> {
        if let Some(cached) = self.extension_method_cache.get(&descriptor) {
            return cached.clone();
        }
        let extensions = self
            .reachable_modules(descriptor.requester)
            .into_iter()
            .flat_map(|owner| {
                self.modules
                    .get_entry(owner)
                    .extensions()
                    .iter()
                    .with_ids()
                    .filter_map(move |(id, extension)| {
                        extension
                            .methods
                            .iter()
                            .any(|method| method.name == descriptor.method_name)
                            .then_some(Owned { owner, term: id })
                    })
            })
            .collect::<Vec<_>>();
        self.extension_method_cache
            .insert(descriptor, extensions.clone());
        extensions
    }

    pub fn reachable_modules(&self, requester: FileId) -> Vec<FileId> {
        if let Some(cached) = self.reachable_modules_cache.get(&requester) {
            return cached.clone();
        }
        let mut visited = std::collections::HashSet::new();
        let modules = self
            .modules
            .find_all_in_modules(requester, &|module| Some(module.id))
            .into_iter()
            .map(|(_, id)| id)
            .filter(|id| visited.insert(*id))
            .collect::<Vec<_>>();
        self.reachable_modules_cache
            .insert(requester, modules.clone());
        modules
    }
}
