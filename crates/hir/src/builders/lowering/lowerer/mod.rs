mod components;
mod declarations;
mod types;

use common::{Spanned, pool::DedupPoolId};
use dashmap::DashMap;
use module_loader::{ASTType, FileId, Modules};
use slynx_parser::{Type, TypeContext};

use crate::{Owned, Result, builders::lowering::lookup::ASTLookup, term::TermId};
pub use declarations::*;

pub struct ASTLowerer<'a> {
    pub(crate) lookup: ASTLookup<'a>,
    pub(super) lowered_types: DashMap<ASTType, Owned<TermId>>,
}

impl<'a> ASTLowerer<'a> {
    pub fn new(modules: &'a Modules<'a>) -> Self {
        Self {
            lookup: ASTLookup::new(modules),
            lowered_types: DashMap::new(),
        }
    }

    pub fn lower_type(
        &self,
        queue: &crate::builders::HirQueueBuilder<'a>,
        requester: FileId,
        ty: Spanned<DedupPoolId<Type>>,
        context: &TypeContext,
    ) -> Result<Owned<TermId>> {
        self.lower_ast_type(
            queue,
            types::TypeLoweringDescriptor {
                requester,
                ty,
                context,
                self_substitute: None,
            },
        )
    }

    pub(crate) fn lower_type_with_self(
        &self,
        queue: &crate::builders::HirQueueBuilder<'a>,
        requester: FileId,
        ty: Spanned<DedupPoolId<Type>>,
        context: &TypeContext,
        self_substitute: TermId,
    ) -> Result<Owned<TermId>> {
        self.lower_ast_type(
            queue,
            types::TypeLoweringDescriptor {
                requester,
                ty,
                context,
                self_substitute: Some(self_substitute),
            },
        )
    }

    pub(crate) fn resolve_signature_of_function(
        &self,
        queue: &crate::builders::HirQueueBuilder<'a>,
        owner: FileId,
        function: &slynx_parser::FuncDeclaration,
    ) -> Result<TermId> {
        let context = TypeContext::new(&function.type_params);
        let return_type = self
            .lower_type(queue, owner, function.return_type, &context)?
            .term;
        let arguments = function
            .args
            .iter()
            .map(|argument| {
                self.lower_type(queue, owner, argument.data.kind, &context)
                    .map(|owned| owned.term)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(queue.hir.types.create_function_type(arguments, return_type))
    }
}
