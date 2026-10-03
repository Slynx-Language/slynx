use common::{
    Spanned, VisibilityModifier,
    pool::{DedupPoolId, PoolId},
};
use module_loader::{ASTType, ASTTypeKind, FileId};
use slynx_parser::{ASTFunction, ExtendDeclaration, InterfaceDeclaration, Type, TypeContext};

use crate::{
    DeclarationId, HIRError, HirExtendDeclaration, HirQueueBuilder, Owned, Result,
    error::InvalidTypeReason, term::TermId,
};

/// Describes the lazy materialization of one concrete interface implementation.
pub(crate) struct InterfaceImplementationDescriptor {
    pub extension: Owned<PoolId<ExtendDeclaration>>,
    pub target: TermId,
}

impl<'a> HirQueueBuilder<'a> {
    pub(crate) fn validate_interface_syntax(&self, requester: FileId) -> Result<()> {
        for owner in self.lowerer.lookup.reachable_modules(requester) {
            let module = self.modules.get_entry(owner);
            for interface in module.interfaces().iter() {
                if !interface.type_args.is_empty() {
                    return Err(HIRError::invalid_type(
                        interface.name,
                        InvalidTypeReason::Unimplemented,
                        interface.span,
                    ));
                }
                if let Some(method) = interface
                    .methods
                    .iter()
                    .find(|method| !method.type_params.is_empty())
                {
                    return Err(HIRError::invalid_type(
                        method.name,
                        InvalidTypeReason::Unimplemented,
                        method.span,
                    ));
                }
            }

            for extension in module.extensions().iter() {
                if !extension.type_args.is_empty() {
                    return Err(HIRError::invalid_type(
                        self.modules.type_name(extension.target.data),
                        InvalidTypeReason::Unimplemented,
                        extension.span,
                    ));
                }
                if let Some(method) = extension
                    .methods
                    .iter()
                    .find(|method| !method.type_params.is_empty())
                {
                    return Err(HIRError::invalid_type(
                        method.name,
                        InvalidTypeReason::Unimplemented,
                        method.span,
                    ));
                }
                self.assert_concrete_type_generic_count(owner, extension.target)?;
                if let Type::Plain(identifier) = self.modules.get_type(extension.interface.data) {
                    if !identifier.generic.is_empty() {
                        return Err(HIRError::invalid_type(
                            identifier.identifier,
                            InvalidTypeReason::Unimplemented,
                            extension.interface.span,
                        ));
                    }
                } else {
                    return Err(HIRError::invalid_type(
                        self.modules.type_name(extension.interface.data),
                        InvalidTypeReason::Unimplemented,
                        extension.interface.span,
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn materialize_interface_implementation(
        &self,
        descriptor: InterfaceImplementationDescriptor,
    ) -> Result<DeclarationId<HirExtendDeclaration>> {
        let cache_key = (
            descriptor.extension.owner,
            descriptor.extension.term,
            descriptor.target,
        );
        if let Some(cached) = self.interface_implementations.get(&cache_key) {
            return Ok(*cached);
        }

        let extension = self
            .modules
            .get_entry(descriptor.extension.owner)
            .extensions()
            .get(descriptor.extension.term);
        if !extension.type_args.is_empty() {
            return Err(HIRError::invalid_type(
                self.modules.type_name(extension.target.data),
                InvalidTypeReason::Unimplemented,
                extension.span,
            ));
        }
        for method in &extension.methods {
            if !method.type_params.is_empty() {
                return Err(HIRError::invalid_type(
                    method.name,
                    InvalidTypeReason::Unimplemented,
                    method.span,
                ));
            }
        }

        self.assert_concrete_type_generic_count(descriptor.extension.owner, extension.target)?;
        self.assert_concrete_type_generic_count(descriptor.extension.owner, extension.interface)?;

        let interface_name = match self.modules.get_type(extension.interface.data) {
            Type::Plain(identifier) if identifier.generic.is_empty() => identifier.identifier,
            _ => {
                return Err(HIRError::invalid_type(
                    self.modules.type_name(extension.interface.data),
                    InvalidTypeReason::Unimplemented,
                    extension.interface.span,
                ));
            }
        };
        let Some(Owned {
            owner: interface_owner,
            term: interface_id,
        }) = self
            .lowerer
            .lookup
            .find_interface(interface_name, descriptor.extension.owner)
        else {
            return Err(HIRError::invalid_type(
                interface_name,
                InvalidTypeReason::Unimplemented,
                extension.interface.span,
            ));
        };

        let interface_ast_type = ASTType {
            owner: interface_owner,
            content: ASTTypeKind::Interface(interface_id),
        };
        let interface_declaration = self
            .modules
            .get_entry(interface_owner)
            .interfaces()
            .get(interface_id);
        self.validate_extension_methods(
            descriptor.extension.owner,
            extension,
            interface_owner,
            interface_declaration,
            descriptor.target,
        )?;

        let interface_term = self
            .lowerer
            .materialize_interface_definition(self, interface_ast_type, extension.interface.span)?
            .term;
        let mut methods = Vec::with_capacity(extension.methods.len());
        for method in &extension.methods {
            let declaration_name = self.hir.intern_name(&format!(
                "__interface_impl_{}_{}_{}",
                descriptor.extension.owner.as_raw(),
                descriptor.extension.term.as_raw(),
                self.hir.get_name(method.name)
            ));
            let declaration_id = self.insert_method_declaration(
                descriptor.extension.owner,
                method,
                descriptor.target,
                VisibilityModifier::Public,
                false,
                declaration_name,
                false,
            )?;
            methods.push((method.name, declaration_id));
        }

        let declaration = HirExtendDeclaration {
            target: descriptor.target,
            interface: interface_term,
            methods,
        };
        let id = {
            let file = self
                .hir
                .store
                .get_or_create_file(descriptor.extension.owner);
            let local = file
                .declarations
                .declarations
                .extensions
                .insert(declaration);
            DeclarationId::new(descriptor.extension.owner, local)
        };
        self.hir
            .types
            .methods
            .create_extension(descriptor.target, id);
        self.interface_implementations.insert(cache_key, id);
        Ok(id)
    }

    fn validate_extension_methods(
        &self,
        extension_file: FileId,
        extension: &ExtendDeclaration,
        interface_file: FileId,
        interface: &InterfaceDeclaration,
        target: TermId,
    ) -> Result<()> {
        if !interface.type_args.is_empty() {
            return Err(HIRError::invalid_type(
                interface.name,
                InvalidTypeReason::Unimplemented,
                interface.span,
            ));
        }
        for method in &interface.methods {
            if !method.type_params.is_empty() {
                return Err(HIRError::invalid_type(
                    method.name,
                    InvalidTypeReason::Unimplemented,
                    method.span,
                ));
            }
        }

        let mut seen = std::collections::HashSet::new();
        for method in &extension.methods {
            if !seen.insert(method.name) {
                return Err(HIRError::already_defined(method.name, method.span));
            }
        }

        for required in &interface.methods {
            let Some(implementation) = extension
                .methods
                .iter()
                .find(|method| method.name == required.name)
            else {
                return Err(HIRError::method_not_found(required.name, extension.span));
            };
            let expected = self.method_signature(interface_file, required, target)?;
            let actual = self.method_signature(extension_file, implementation, target)?;
            if expected != actual {
                return Err(HIRError::unexpected_type(
                    actual,
                    expected,
                    implementation.span,
                ));
            }
        }
        Ok(())
    }

    fn method_signature<T: ASTFunction>(
        &self,
        file_id: FileId,
        method: &T,
        self_type: TermId,
    ) -> Result<TermId> {
        let context = TypeContext::new(method.type_params());
        let lower_type = |ty: Spanned<DedupPoolId<Type>>| {
            self.lowerer
                .lower_type_with_self(self, file_id, ty, &context, self_type)
                .map(|owned| owned.term)
        };
        let args = method
            .arguments()
            .iter()
            .map(|arg| lower_type(arg.data.kind))
            .collect::<Result<Vec<_>>>()?;
        let ret = lower_type(method.return_type())?;
        Ok(self.hir.types.create_function_type(args, ret))
    }
}
