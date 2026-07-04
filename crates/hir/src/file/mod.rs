//! Module idealized to handle HirFiles, which is a file on the slynx codebase

mod declarations;
use std::ops::Deref;

use common::pool::PoolId;
use module_loader::FileId;

use crate::{
    DeclarationId, HirAliasDeclaration, HirComponentDeclaration, HirFunctionDeclaration,
    HirObjectDeclaration, HirStaticDeclaration, HirStylesheetDeclaration, SymbolPointer,
    context::ScopeContext, file::declarations::FileDeclarations,
};

#[derive(Debug)]
pub struct HirFile {
    pub file: FileId,
    pub declarations: FileDeclarations,
    pub scopes: ScopeContext,
}

macro_rules! create_methods {
    ($($name: ident = $target:ident($typ:ty)),* $(,)?) => {
        $(
            pub(crate) fn $name(&self, arg: $typ) -> DeclarationId<$typ> {
                let out = self.$target(arg);
                DeclarationId::new(self.file, out)
            }
        )*
    };
}

impl HirFile {
    pub fn new(file: FileId) -> Self {
        HirFile {
            file,
            declarations: FileDeclarations::new(),
            scopes: ScopeContext::new(),
        }
    }

    create_methods!(
        create_function = insert_at_functions(HirFunctionDeclaration),
        create_object = insert_at_objects(HirObjectDeclaration),
        create_component = insert_at_components(HirComponentDeclaration),
        create_static = insert_at_statik(HirStaticDeclaration),
    );

    pub fn find_function_with_name(
        &self,
        name: SymbolPointer,
    ) -> Option<DeclarationId<HirFunctionDeclaration>> {
        self.declarations
            .declarations
            .functions
            .iter()
            .position(|f| f.name == name)
            .map(|idx| DeclarationId::new(self.file, PoolId::new(idx as u32)))
        //since internally its just an index, this gets the id properly.
    }

    pub fn find_component_with_name(
        &self,
        name: SymbolPointer,
    ) -> Option<DeclarationId<HirComponentDeclaration>> {
        self.declarations
            .components
            .iter()
            .with_ids()
            .find(|(_, c)| c.name == name)
            .map(|(id, _)| DeclarationId::new(self.file, id))
    }
}
impl Deref for HirFile {
    type Target = FileDeclarations;
    fn deref(&self) -> &Self::Target {
        &self.declarations
    }
}
