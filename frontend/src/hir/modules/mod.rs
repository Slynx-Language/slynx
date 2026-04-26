use common::SymbolsModule;

use crate::hir::modules::{
    declarations::DeclarationsModule,
    scopes::ScopeModule,
    types::{BUILTIN_NAMES, TypesModule},
};

pub mod declarations;
pub mod scopes;
pub mod symbols;
pub mod types;

#[derive(Debug, Default)]
pub struct HirModules {
    pub declarations_module: DeclarationsModule,
    pub symbols_module: SymbolsModule,
    pub types_module: TypesModule,
    pub scope_module: ScopeModule,
}

impl HirModules {
    pub fn new() -> Self {
        let mut symbols = SymbolsModule::new();
        let builtins = BUILTIN_NAMES.map(|v| symbols.intern(v));
        Self {
            declarations_module: DeclarationsModule::new(),
            symbols_module: SymbolsModule::new(),
            types_module: TypesModule::new(&builtins),
            scope_module: ScopeModule::new(),
        }
    }
}
