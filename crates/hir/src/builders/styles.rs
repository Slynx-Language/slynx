use module_loader::FileId;

use crate::{
    DeclarationId, HirStylesheetDeclaration, Result, SymbolPointer,
    builders::HirQueueBuilder,
    context::HirSymbol,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};

impl<'a> HirQueueBuilder<'a> {
    /// Lazily enqueues a stylesheet for processing. Called when a component or
    /// function body references a style. If the stylesheet hasn't been hoisted
    /// yet, it creates the declaration and registers it.
    #[allow(dead_code)]
    pub(crate) fn enqueue_stylesheet(
        &self,
        name: SymbolPointer,
        stylesheet: &'a slynx_parser::StyleSheet,
        file_id: FileId,
    ) -> Result<DeclarationId<HirStylesheetDeclaration>> {
        // Check if already registered first to avoid re-computation
        if let Some(existing) = self
            .hir
            .symbols_registry
            .get_style(HirSymbol::new(file_id, name))
        {
            return Ok(existing);
        }

        // Build style type before creating the declaration
        let node = self.get_node(file_id);
        let args: Result<Vec<_>> = stylesheet
            .args
            .iter()
            .map(|arg| node.find_type(arg.data.kind).map(|v| v.1))
            .collect();
        let args = args?;
        let ty = self.hir.types_module.create_style_type(name, args);

        let id = {
            let decl = HirStylesheetDeclaration {
                name,
                usages: Vec::new(),
                args: Default::default(),
                statements: Vec::new(),
                ty,
                visibility: stylesheet.visibility,
                external: false,
                attributes: Vec::new(),
            };
            let file = self.hir.get_or_create_file(file_id);
            let id = file.create_stylesheet(decl);
            self.hir
                .symbols_registry
                .register_style(HirSymbol::new(file_id, name), id);
            id
        };

        let decl_id = AnyDeclarationId::new(file_id, AnyLocalDeclarationId::Style(id.local_id));
        let attrs =
            super::attributes::process_attributes(self.hir, &stylesheet.attributes, decl_id);
        if !attrs.is_empty() {
            self.hir
                .get_file_mut(file_id)
                .declarations
                .styles
                .get_mut(id.local_id)
                .attributes = attrs;
        }

        Ok(id)
    }

    /// Finds a stylesheet by name, hoisting it lazily from the AST if needed.
    #[allow(dead_code)]
    pub(crate) fn find_style_named(
        &self,
        name: SymbolPointer,
        requester: FileId,
    ) -> Option<DeclarationId<HirStylesheetDeclaration>> {
        if let Some(style) = self
            .hir
            .symbols_registry
            .get_style(HirSymbol::new(requester, name))
        {
            return Some(style);
        }

        let entry = self.modules.get_entry(requester);
        let style_sheet = entry
            .style()
            .iter()
            .find(|s| self.modules.get_type(s.name.data).identifier == name)?;

        self.enqueue_stylesheet(name, style_sheet, requester).ok()
    }
}
