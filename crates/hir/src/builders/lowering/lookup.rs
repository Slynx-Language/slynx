use common::{FrontendSymbol, SymbolPointer};
use module_loader::{ASTType, FileId, Modules};
use slynx_parser::StaticDeclaration;

pub struct ASTLookup<'a> {
    modules: &'a Modules<'a>,
}

impl<'a> ASTLookup<'a> {
    pub fn new(modules: &'a Modules<'a>) -> Self {
        Self { modules }
    }

    pub fn find_type(
        &self,
        requester: FileId,
        name: SymbolPointer<FrontendSymbol>,
    ) -> Option<ASTType> {
        self.modules.find_type(requester, name)
    }

    pub fn find_function(
        &self,
        name: SymbolPointer<FrontendSymbol>,
        requester: FileId,
    ) -> Option<(FileId, usize)> {
        self.modules.find_function_declaration(name, requester)
    }

    pub fn find_interface(
        &self,
        name: SymbolPointer<FrontendSymbol>,
        requester: FileId,
    ) -> Option<(FileId, usize)> {
        self.modules.find_interface_declaration(name, requester)
    }

    pub fn find_enum_variant(
        &self,
        name: SymbolPointer<FrontendSymbol>,
        requester: FileId,
    ) -> Option<(
        FileId,
        common::pool::PoolId<slynx_parser::EnumDeclaration>,
        usize,
    )> {
        self.modules.find_enum_variant(name, requester)
    }

    pub fn find_static(
        &self,
        name: SymbolPointer<FrontendSymbol>,
        requester: FileId,
    ) -> Option<(FileId, &'a StaticDeclaration)> {
        self.modules.find_static_declaration(name, requester)
    }

    pub fn find_all_in_modules<T, K>(
        &self,
        name: K,
        requester: FileId,
        finder: &dyn Fn(&module_loader::SourceNode, &K) -> Option<T>,
    ) -> Vec<(FileId, T)> {
        self.modules.find_all_in_modules(name, requester, finder)
    }
}
