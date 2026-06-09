//! Module idealized to handle HirFiles, which is a file on the slynx codebase

use common::Span;

use crate::{
    HIRError, HirDeclaration, Result, SymbolPointer, TypeId, VariableId,
    context::{DeclarationsContext, ScopeContext},
    id::LocalDeclId,
    module_loader::FileId,
};

#[derive(Debug)]
pub struct HirFile {
    pub file: FileId,
    pub declarations: DeclarationsContext,
    pub scopes: ScopeContext,
}

impl HirFile {
    pub fn new(file: FileId) -> Self {
        HirFile {
            file,
            declarations: DeclarationsContext::new(),
            scopes: ScopeContext::new(),
        }
    }
    pub fn create_declaration(&mut self, mut decl: HirDeclaration) {
        let id = decl.id.local_id.as_raw();
        let len = self.declarations.declarations.count();
        assert!(
            id == len,
            "create_declaration: local_id={id} != declarations.len()={len}, file_id={:?}",
            decl.id.file_id,
        );
        decl.visibility = self.declarations.get_visibility(decl.id.local_id);
        self.declarations.declarations.push(decl);
    }
    ///Creates a imutable variable with the given `name` on this file. Its registering on the types context and on symbol resolver is hir responsability
    pub(crate) fn create_variable(
        &mut self,
        symbol: SymbolPointer,
        span: &Span,
        mutable: bool,
    ) -> Result<VariableId> {
        if self.scopes.get_name(&symbol).is_some() {
            Err(HIRError::already_defined(symbol, *span))
        } else {
            let v = VariableId::new();
            self.scopes.create_name(symbol, v, mutable);
            Ok(v)
        }
    }
    pub fn declarations(&self) -> &boxcar::Vec<HirDeclaration> {
        &self.declarations.declarations
    }
    pub fn get_declaration_type(&self, id: LocalDeclId) -> TypeId {
        self.declarations.get_declaration_type(id)
    }
}
