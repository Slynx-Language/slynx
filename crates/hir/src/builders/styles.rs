use module_loader::FileId;
use slynx_parser::TypeContext;

use crate::{
    DeclarationId, HirStylesheetDeclaration, Result, SymbolPointer,
    builders::HirQueueBuilder,
    context::HirSymbol,
    id::AnyLocalDeclarationId,
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
            .map(|arg| {
                node.find_type(arg.data.kind, &TypeContext::new(&stylesheet.type_params))
                    .map(|v| v.1)
            })
            .collect();
        let args = args?;
        let ty = self.hir.types.create_style_type(name, args);

        let id = {
            let decl = HirStylesheetDeclaration {
                name,
                generics: stylesheet.type_params.clone(),
                usages: Vec::new(),
                args: Default::default(),
                statements: Vec::new(),
                ty,
                visibility: stylesheet.visibility,
                external: false,
                attributes: Vec::new(),
            };
            let file = self.hir.store.get_or_create_file(file_id);
            let id = file.create_stylesheet(decl);
            self.hir
                .symbols_registry
                .register_style(HirSymbol::new(file_id, name), id);
            id
        };

        self.attach_attributes(
            file_id,
            AnyLocalDeclarationId::Style(id.local_id),
            &stylesheet.attributes,
        )?;

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
        let style_sheet = entry.style().iter().find(|s| s.name == name)?;

        self.enqueue_stylesheet(name, style_sheet, requester).ok()
    }
}
