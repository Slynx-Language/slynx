//! Module idealized to handle HirFiles, which is a file on the slynx codebase

use crate::{
    HirDeclaration,
    module_loader::FileId,
    modules::{DeclarationsModule, ScopeModule},
};

#[derive(Debug)]
pub struct HirFile {
    pub(crate) file: FileId,
    pub(crate) declarations: DeclarationsModule,
    scopes: ScopeModule,
}

impl HirFile {
    pub fn create_declaration(&mut self, decl: HirDeclaration) {
        debug_assert!(decl.id.local_id.as_raw() == self.declarations.declarations.len());
        self.declarations.declarations.push(decl);
    }
}
