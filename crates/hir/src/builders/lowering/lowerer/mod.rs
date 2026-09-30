mod components;
mod declarations;
mod types;

use common::{Spanned, pool::DedupPoolId};
use dashmap::DashMap;
use module_loader::{ASTType, FileId, Modules};
use slynx_parser::{Type, TypeContext};

use crate::{Owned, Result, builders::lowering::lookup::ASTLookup, term::TermId};

pub use components::*;
pub use declarations::*;
pub use types::*;

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
        declarations::resolve_signature_of_function(self, queue, owner, function)
    }
}
