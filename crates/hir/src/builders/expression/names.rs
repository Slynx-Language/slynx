use common::{Span, Spanned};

use crate::{
    DeclarationId, HIRError, HirExpression, HirExpressionKind, HirFunctionDeclaration,
    HirStaticDeclaration, Result, SymbolPointer, VariableId, builders::HirQueueBuilder,
    context::HirSymbol,
};

use super::ExpressionBuilder;

pub(crate) enum HirName {
    Variable(VariableId),
    Static(DeclarationId<HirStaticDeclaration>),
}

impl ExpressionBuilder {
    /// Resolves a name to either a local variable or a static declaration.
    /// Component method builders can extend this by resolving component fields
    /// before falling back to statics.
    pub(crate) fn resolve_name(
        &self,
        queue: &HirQueueBuilder,
        ptr: SymbolPointer,
        span: Span,
    ) -> Result<HirName> {
        if let Some(var) = self.names.get(&ptr).cloned() {
            Ok(HirName::Variable(var))
        } else if let Some((file_owner, statik)) = queue.find_static_declaration(ptr, self.file()) {
            let id = queue.enqueue_static(statik, queue.get_node(file_owner))?;
            Ok(HirName::Static(id))
        } else {
            Err(HIRError::name_unrecognized(ptr, span))
        }
    }

    pub(super) fn lookup_function(
        &self,
        queue: &HirQueueBuilder<'_>,
        name: Spanned<SymbolPointer>,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        let identifier = name.data;

        if let Some(func) = queue
            .hir
            .find_function_by_symbol(HirSymbol::new(self.file(), identifier))
        {
            Ok(func)
        } else if let Some(func) = queue
            .hir
            .get_file(self.file())
            .find_function_with_name(identifier)
        {
            Ok(func)
        } else {
            Err(HIRError::name_unrecognized(identifier, name.span))
        }
    }

    pub(super) fn build_identifier(
        &self,
        queue: &HirQueueBuilder,
        name: SymbolPointer,
        expression_span: Span,
    ) -> Result<HirExpression> {
        match self.resolve_name(queue, name, expression_span)? {
            HirName::Variable(v) => {
                let ty = *self
                    .variables_types
                    .get(&v)
                    .expect("Expected variable to have a type defined on this builder");
                Ok(HirExpression {
                    ty,
                    kind: HirExpressionKind::Identifier(v),
                })
            }
            HirName::Static(s) => {
                let ty = queue.hir.get_static(s).ty;
                Ok(HirExpression {
                    ty,
                    kind: HirExpressionKind::Static { id: s },
                })
            }
        }
    }
}
