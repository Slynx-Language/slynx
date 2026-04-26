use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use common::{Span, SymbolPointer, SymbolsModule};

use crate::hir::{
    VariableId,
    error::HIRError,
    modules::{
        declarations::DeclarationsModule,
        scopes::ScopeModule,
        types::{BUILTIN_NAMES, TypesModule},
    },
};

pub mod declarations;
pub mod scopes;
pub mod symbols;
pub mod types;

#[derive(Debug, Default)]
pub struct SymbolsResolver {
    module: SymbolsModule,
    /// Tracks the original source-level symbol for each variable id.
    variable_names: HashMap<VariableId, SymbolPointer>,
}

impl Deref for SymbolsResolver {
    type Target = SymbolsModule;
    fn deref(&self) -> &Self::Target {
        &self.module
    }
}
impl DerefMut for SymbolsResolver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.module
    }
}

impl SymbolsResolver {
    pub fn new(module: SymbolsModule) -> Self {
        Self {
            module,
            variable_names: HashMap::new(),
        }
    }
}

#[derive(Debug, Default)]
///A Modules object to handle with creation of symbols, declarations, types, scopes, etc.
///
pub struct HirModules {
    pub declarations_module: DeclarationsModule,
    pub symbols_resolver: SymbolsResolver,
    pub types_module: TypesModule,
    pub scope_module: ScopeModule,
}

impl HirModules {
    pub fn new() -> Self {
        let mut symbols = SymbolsModule::new();
        let builtins = BUILTIN_NAMES.map(|v| symbols.intern(v));
        Self {
            declarations_module: DeclarationsModule::new(),
            symbols_resolver: SymbolsResolver::new(symbols),
            types_module: TypesModule::new(&builtins),
            scope_module: ScopeModule::new(),
        }
    }

    ///Interns the given `s` and returns its logical pointer
    pub fn intern_name(&mut self, s: &str) -> SymbolPointer {
        self.symbols_resolver.intern(s)
    }

    ///Finds some variable based on the given `name`. Checks all the scopes that are there currently
    pub fn find_variable(&self, name: SymbolPointer) -> Option<VariableId> {
        let mut idx = self.scope_module.len() - 1;
        while idx != 0 {
            let scope = &self.scope_module[idx];

            let Some(id) = scope.retrieve_name(&name) else {
                idx -= 1;
                continue;
            };
            return Some(*id);
        }
        None
    }

    pub fn create_variable(
        &mut self,
        name: SymbolPointer,
        mutable: bool,
        ty: super::TypeId,
        span: &Span,
    ) -> Result<VariableId, HIRError> {
        if let Some(_) = self.scope_module.retrieve_name(&name) {
            Err(HIRError::already_defined(name, *span))
        } else {
            let v = VariableId::new();
            self.scope_module.insert_name(name, v, mutable);
            self.types_module.insert_variable(v, ty);
            self.symbols_resolver.variable_names.insert(v, name);
            Ok(v)
        }
    }
}
