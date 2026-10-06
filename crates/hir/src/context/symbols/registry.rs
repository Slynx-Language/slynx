use crate::{
    DeclarationId, HirComponentDeclaration, HirFunctionDeclaration, HirStaticDeclaration, Owned,
    Result, SymbolPointer, id::AnyDeclarationId,
};
use dashmap::{DashMap, DashSet};

///Represents a symbol on the HIR that was found at an specific file and has an specific name
pub type HirSymbol = Owned<SymbolPointer>;

macro_rules! impl_get_or_insert {
    ($($fname:ident: $type:ty => $pname:ident),*$(,)?) => {
        $(
            paste::paste!{
                pub fn [<get_or_insert_ $fname>](&self, key: HirSymbol, make_decl: impl FnOnce() -> Result<DeclarationId<$type>>) -> Result<DeclarationId<$type>> {
                    Ok(*self.$pname.entry(key).or_insert(make_decl()?).value())
                }
            }
        )*
    };
}

#[derive(Debug, Default)]
///A Struct to registry symbols that were hoisted and analyzed, and a way to map them to their actual id on the hir
pub struct SymbolRegistry {
    functions: DashMap<HirSymbol, DeclarationId<HirFunctionDeclaration>>,
    components: DashMap<HirSymbol, DeclarationId<HirComponentDeclaration>>,
    statics: DashMap<HirSymbol, DeclarationId<HirStaticDeclaration>>,

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

    pub fn get_component(&self, name: HirSymbol) -> Option<DeclarationId<HirComponentDeclaration>> {
        self.components.get(&name).map(|v| *v.value())
    }

    impl_get_or_insert!(
        function: HirFunctionDeclaration => functions,
        static: HirStaticDeclaration => statics,
        component: HirComponentDeclaration => components
    );
}
