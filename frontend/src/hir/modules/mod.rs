use common::{ObjectField, Span};

use crate::hir::{DeclarationId, Result, TypeId, VariableId, error::HIRError, model::HirType};

mod declarations;
mod scopes;
mod symbols;
mod types;
pub use declarations::*;
pub use scopes::*;
pub use symbols::*;
pub use types::*;

#[derive(Debug, Default)]
///A Modules object to handle with creation of symbols, declarations, types, scopes, etc.
///
pub struct HirModules {
    pub declarations_module: DeclarationsModule,
    pub symbols_resolver: SymbolsResolver,
    pub types_module: TypesModule,
    pub scope_module: ScopeModule,
}

//specific for naming
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
    ///Retrieves the symbol pointer for the given `s` if it exists, thus, was internalized
    pub fn retrieve_symbol(&self, s: &str) -> Option<SymbolPointer> {
        self.symbols_resolver.retrieve(s).cloned()
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
    ) -> Result<VariableId> {
        if self.scope_module.retrieve_name(&name).is_some() {
            Err(HIRError::already_defined(name, *span))
        } else {
            let v = VariableId::new();
            self.scope_module.insert_name(name, v, mutable);
            self.types_module.insert_variable(v, ty);
            self.symbols_resolver.register_variable(v, name);
            Ok(v)
        }
    }
}

//specific for declarations
impl HirModules {
    ///Creates an type alias with the given `name`. Its initial type is `infer`. Because of hoisting, and so, the type this refers to might be defined after it
    pub fn create_alias(&mut self, target: &str, name: &str) {
        self.symbols_resolver.intern(target);
        let symbol = self.symbols_resolver.intern(name);
        let ty = self.types_module.insert_type(symbol, HirType::Infer);
        self.declarations_module.create_declaration(symbol, ty);
    }

    ///Creates a new declaration with the given `name` and `ty`. returns its symbol, type id, and declaration id.
    pub fn create_declaration(
        &mut self,
        name: &str,
        ty: HirType,
    ) -> (SymbolPointer, TypeId, DeclarationId) {
        let symbol = self.symbols_resolver.intern(name);
        let tyid = self.types_module.insert_type(symbol, ty);
        let decl_id = self.declarations_module.create_declaration(symbol, tyid);
        (symbol, tyid, decl_id)
    }

    pub fn create_object(&mut self, name: &str, fields: &[ObjectField]) {
        let name = self.symbols_resolver.intern(name);
        let def_fields = fields
            .iter()
            .map(|f| self.symbols_resolver.intern(&f.name.name))
            .collect();
        let ty = self
            .types_module
            .insert_unnamed_type(HirType::new_struct(Vec::new()));
        let ty = self.types_module.insert_type(name, HirType::new_ref(ty));
        self.declarations_module.create_object(name, ty, def_fields);
    }
    ///Retrieves the declaration ID and type based on the given `symbol`
    pub fn get_declaration_by_name(
        &self,
        symbol: &SymbolPointer,
    ) -> Option<(DeclarationId, TypeId)> {
        self.declarations_module
            .retrieve_declaration_data_by_name(symbol)
    }
}

//specific for scopes
impl HirModules {
    pub fn enter_scope(&mut self) -> &mut HIRScope {
        self.scope_module.enter_scope()
    }
    pub fn exit_scope(&mut self) -> &mut HIRScope {
        self.scope_module.enter_scope()
    }
}

//specific for types
impl HirModules {
    pub fn find_type_by_name(&mut self, name: &str, span: &Span) -> Result<&TypeId> {
        let name = self.symbols_resolver.intern(name);
        self.types_module
            .get_id(&name)
            .ok_or(HIRError::name_unrecognized(name, *span))
    }
}
