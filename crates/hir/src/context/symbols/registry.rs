use dashmap::{DashMap, DashSet};
use module_loader::FileId;

use crate::{
    DeclarationId, HirFunctionDeclaration, HirStaticDeclaration, SymbolPointer,
    id::AnyDeclarationId,
};

#[derive(Debug, PartialEq, Eq, Hash)]
///Represents a symbol on the HIR that was found at an specific file and has an specific name
pub struct HirSymbol {
    ///The file that contains the given `name`
    file: FileId,
    ///The actual name that appeared in the file with this file id
    name: SymbolPointer,
}

impl HirSymbol {
    pub fn new(file: FileId, name: SymbolPointer) -> Self {
        Self { file, name }
    }
}

macro_rules! impl_get_or_insert {
    ($($fname:ident: $type:ty => $pname:ident),*$(,)?) => {
        $(
            paste::paste!{
                pub fn [<get_or_insert_ $fname>](&self, key: HirSymbol, make_decl: impl FnOnce() -> DeclarationId<$type>) -> DeclarationId<$type> {
                    *self.$pname.entry(key).or_insert_with(make_decl).value()
                }
            }
        )*
    };
}

#[derive(Debug, Default)]
///A Struct to registry symbols that were hoisted and analyzed, and a way to map them to their actual id on the hir
pub struct SymbolRegistry {
    // Registros globais de IDs
    functions: DashMap<HirSymbol, DeclarationId<HirFunctionDeclaration>>,
    statics: DashMap<HirSymbol, DeclarationId<HirStaticDeclaration>>,

    // Estados de processamento
    hoisted: DashSet<HirSymbol>,
    analyzed: DashSet<AnyDeclarationId>,
}

impl SymbolRegistry {
    pub fn hoist(&self, symbol: HirSymbol) -> bool {
        self.hoisted.insert(symbol)
    }

    pub fn analyze(&self, id: AnyDeclarationId) -> bool {
        self.analyzed.insert(id)
    }

    pub fn get_function(&self, name: HirSymbol) -> Option<DeclarationId<HirFunctionDeclaration>> {
        self.functions.get(&name).map(|v| *v.value())
    }

    impl_get_or_insert!(
        function: HirFunctionDeclaration => functions,
        static: HirStaticDeclaration => statics
    );
}
