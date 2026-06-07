//! Module idealized to handle HirFiles, which is a file on the slynx codebase

use std::ops::{Index, IndexMut};

use crate::{
    DeclarationId, HirDeclaration,
    module_loader::FileId,
    modules::{DeclarationsModule, ScopeModule},
};

#[derive(Debug)]
pub struct HirFile {
    pub(crate) file: FileId,
    pub(crate) declarations: DeclarationsModule,
    scopes: ScopeModule,
}
