use common::{Span, Spanned, pool::DedupPoolId};
use module_loader::{ASTTypeKind, FileId};
use slynx_parser::{ASTExpression, GenericIdentifier, Type, TypeContext};

use crate::{
    HIRError, HirQueueBuilder, Owned, Result,
    arrays::ArrayTerm,
    builders::lowering::{ASTLowerer, lowerer::declarations::LowerTypeDeclarationDescriptor},
    error::InvalidTypeReason,
    term::{Term, TermId},
    vector::VectorTerm,
};

pub(super) struct TypeLoweringDescriptor<'a> {
    ///The file requesting the lowering.
    pub requester: FileId,
    ///The ast type being lowered
    pub ty: Spanned<DedupPoolId<Type>>,
    ///The generic context for resolving generic types.
    pub context: &'a TypeContext<'a>,
    ///The concrete term that replaces `Self`, without affecting surrounding types.
    pub self_substitute: Option<TermId>,
}

impl<'a> ASTLowerer<'a> {
    ///Lowers the given plain type `generic` into a [`OwnedTerm`].
    ///If the given `generic` does not contain generic parameters in it, which can be generated with something like: `Thing`, and it's found in the given `context`, the it returns an owned term that represents that position.
    ///For example `func f<T,K>(): K -> ...;` when using 'K' the given `generic` is simply `K` and since it's the second parameter on the context(`[T,K]`), then it will return a term that represents the second generic parameter position.
    ///Otherwise, if the `generic` is not found in the `context`, it will try to find the given `generic` starting by the given `requester` file. For example the `Thing` type above, it will try to search for it starting by the file
    ///that requested this operation. If the given `generic` contains generic parameters, then the output term will be updated to contain these generics. So such as func `f<T,K>: K {let var: Vec<Thing> = ...;};`
    ///Vec<Thing> is a generic type that contains a `Thing` type, so the output term will properly represent this Vec type with the generic Thing inside id.
    fn lower_plain_type(
        &self,
        queue: &HirQueueBuilder<'a>,
        requester: FileId,
        generic: &GenericIdentifier,
        context: &TypeContext,
        self_substitute: Option<TermId>,
        span: Span,
    ) -> Result<Owned<TermId>> {
        if generic.generic.is_empty()
            && let Some(target) = context
                .generic_names
                .iter()
                .position(|name| *name == generic.identifier)
        {
            return Ok(Owned {
                owner: requester,
                term: queue
                    .hir
                    .types
                    .create_type(Term::new_variable_type(target as u8, generic.identifier)),
            });
        }
        let ast_type = self
            .lookup
            .find_type(requester, generic.identifier)
            .ok_or_else(|| HIRError::type_unrecognized(generic.identifier, span))?;
        let Owned { owner, term: ty } = self.materialize_type_declaration(
            queue,
            LowerTypeDeclarationDescriptor {
                ast_type,
                context: &context,
                span,
            },
        )?;
        if generic.generic.is_empty() {
            return Ok(Owned { owner, term: ty });
        }

        let ty_view = queue.hir.view(ty);
        let deref = ty_view.dereference();
        if deref.is_struct().is_none()
            && deref.is_component().is_none()
            && deref.is_enum().is_none()
        {
            return Ok(Owned { owner, term: ty });
        }
        let args = generic
            .generic
            .iter()
            .map(|arg| {
                self.lower_ast_type(
                    queue,
                    TypeLoweringDescriptor {
                        requester,
                        ty: span.make_spanned(arg.data),
                        context,
                        self_substitute,
                    },
                )
                .map(|ownedterm| ownedterm.term)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Owned {
            owner,
            term: queue.hir.types.create_type(Term::application(ty, args)),
        })
    }
    ///Lowers an AST type recursively, substituting only the `Self` identifier
    ///while preserving wrappers and generic applications around it.
    ///Given a type from the AST, defined at `descriptor.ty`, it tries to find it on the HIR and returns so. If not found, it searches for some with that name on the AST and materialized it.
    ///If any type on the AST is defined as `Self`, then it replaces it with the given `descriptor.self_substitute` if some. Otherwise, it returns the `Self` identifier as is.
    pub(super) fn lower_ast_type(
        &self,
        queue: &HirQueueBuilder<'a>,
        descriptor: TypeLoweringDescriptor,
    ) -> Result<Owned<TermId>> {
        let cache_key = super::TypeLoweringCacheKey {
            requester: descriptor.requester,
            ty: descriptor.ty.data,
            generic_names: descriptor.context.generic_names.to_vec(),
            self_substitute: descriptor.self_substitute,
        };
        if let Some(lowered) = self.lowered_ast_types.get(&cache_key) {
            return Ok(lowered.value().clone());
        }
        let lowered = self.lower_ast_type_uncached(queue, descriptor)?;
        self.lowered_ast_types.insert(cache_key, lowered.clone());
        Ok(lowered)
    }

    fn lower_ast_type_uncached(
        &self,
        queue: &HirQueueBuilder<'a>,
        descriptor: TypeLoweringDescriptor,
    ) -> Result<Owned<TermId>> {
        let real = queue.modules.get_type(descriptor.ty.data);
        let self_symbol = queue.hir.intern_name("Self");
        match real {
            Type::Plain(generic)
                if let Some(substitute) = descriptor.self_substitute
                    && generic.generic.is_empty()
                    && generic.identifier == self_symbol =>
            {
                Ok(Owned {
                    owner: descriptor.requester,
                    term: substitute,
                })
            }
            Type::Plain(generic)
                if !generic.generic.is_empty()
                    && matches!(
                        self.lookup
                            .find_type(descriptor.requester, generic.identifier)
                            .map(|ty| ty.content),
                        Some(ASTTypeKind::Interface(_))
                    ) =>
            {
                return Err(HIRError::invalid_type(
                    generic.identifier,
                    InvalidTypeReason::Unimplemented,
                    descriptor.ty.span,
                ));
            }
            Type::Plain(generic) => self.lower_plain_type(
                queue,
                descriptor.requester,
                generic,
                descriptor.context,
                descriptor.self_substitute,
                descriptor.ty.span,
            ),
            Type::Array(t, len) => {
                let Owned { owner, term: ty } = self.lower_ast_type(
                    queue,
                    TypeLoweringDescriptor {
                        requester: descriptor.requester,
                        ty: descriptor.ty.span.make_spanned(*t),
                        context: descriptor.context,
                        self_substitute: descriptor.self_substitute,
                    },
                )?;
                let len = match queue.modules.get_expr(*len) {
                    ASTExpression::IntLiteral(i) => *i as usize,
                    _ => unimplemented!(
                        "Array length can only be used as integers at the moment. It is idealized to be used in comptime in the future"
                    ),
                };
                let array = queue.hir.types.create_extension_type(ArrayTerm);
                let len = queue
                    .hir
                    .types
                    .create_type(Term::const_usize_type(len as usize));
                let ty = queue
                    .hir
                    .types
                    .create_type(Term::application(array, vec![ty, len]));
                Ok(Owned { owner, term: ty })
            }
            Type::Vector(t) => {
                let Owned { owner, term: ty } = self.lower_ast_type(
                    queue,
                    TypeLoweringDescriptor {
                        requester: descriptor.requester,
                        ty: descriptor.ty.span.make_spanned(*t),
                        context: descriptor.context,
                        self_substitute: descriptor.self_substitute,
                    },
                )?;
                let array = queue.hir.types.create_extension_type(VectorTerm);
                let ty = queue
                    .hir
                    .types
                    .create_type(Term::application(array, vec![ty]));
                Ok(Owned { owner, term: ty })
            }
            Type::Reference(t) => {
                let Owned { owner, term: ty } = self.lower_ast_type(
                    queue,
                    TypeLoweringDescriptor {
                        requester: descriptor.requester,
                        ty: descriptor.ty.span.make_spanned(*t),
                        context: descriptor.context,
                        self_substitute: descriptor.self_substitute,
                    },
                )?;

                let ty = queue.hir.types.create_type(Term::reference(ty));
                Ok(Owned { owner, term: ty })
            }
            Type::MutableReference(t) => {
                let Owned { owner, term: ty } = self.lower_ast_type(
                    queue,
                    TypeLoweringDescriptor {
                        requester: descriptor.requester,
                        ty: descriptor.ty.span.make_spanned(*t),
                        context: descriptor.context,
                        self_substitute: descriptor.self_substitute,
                    },
                )?;
                let ty = queue.hir.types.create_type(Term::mutable_reference(ty));
                Ok(Owned { owner, term: ty })
            }
        }
    }
}
