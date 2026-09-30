use common::Span;
use module_loader::{ASTType, ASTTypeKind, FileId};

use crate::{
    DeclarationId, DescriptorId, HirFunctionDeclaration, HirQueueBuilder, Result, SymbolPointer,
    term::{TermId, TermNode},
};

impl HirQueueBuilder<'_> {
    /// Lazily materializes an inherent method declared on a struct.
    pub(crate) fn resolve_method(
        &self,
        file_id: FileId,
        struct_ty: TermId,
        method_name: SymbolPointer,
        _span: Span,
    ) -> Result<Option<DeclarationId<HirFunctionDeclaration>>> {
        let struct_id = match self.hir.types[struct_ty].node() {
            TermNode::Data(DescriptorId::Struct(id)) => *id,
            _ => return Ok(None),
        };
        let struct_name = self.hir.types.get_struct_name(struct_id);

        let Some(ASTType {
            owner,
            content: ASTTypeKind::Struct(declaration),
        }) = self.lowerer.lookup.find_type(file_id, struct_name)
        else {
            return Ok(None);
        };
        let object = self.modules.get_entry(owner).object().get(declaration);
        let Some(method) = object
            .methods
            .iter()
            .find(|method| method.method_name == method_name)
        else {
            return Ok(None);
        };

        let declaration_name = self.hir.intern_name(&format!(
            "{}_{}",
            self.hir.get_name(method_name),
            self.hir.get_name(struct_name)
        ));
        self.insert_method_declaration(
            owner,
            method,
            struct_ty,
            object.visibility,
            object.external,
            declaration_name,
            true,
        )
        .map(Some)
    }
}
