use common::{
    Span, Spanned, VisibilityModifier,
    pool::{DedupPoolId, PoolId},
};
use module_loader::{ASTType, ASTTypeKind, FileId};
use slynx_parser::{ASTFunction, ExtendDeclaration, Type, TypeContext};

use crate::{
    DeclarationId, HIRError, HirExtendDeclaration, HirFunctionDeclaration, HirQueueBuilder, Result,
    SymbolPointer, error::InvalidTypeReason, term::TermId,
};

impl<'a> HirQueueBuilder<'a> {
    pub(crate) fn prepare_interfaces(&self) -> Result<()> {
        for module in self.modules.entries() {
            for interface in module.interfaces().iter() {
                if !interface.type_args.is_empty() {
                    return Err(HIRError::invalid_type(
                        interface.name,
                        InvalidTypeReason::IncorrectUsage,
                        interface.span,
                    ));
                }
                for method in &interface.methods {
                    if !method.type_params.is_empty() {
                        return Err(HIRError::invalid_type(
                            method.name,
                            InvalidTypeReason::IncorrectUsage,
                            method.span,
                        ));
                    }
                }
            }
        }

        for module in self.modules.entries() {
            for (index, extension) in module.extensions().iter().enumerate() {
                self.prepare_extension(module.id, index, extension)?;
            }
        }

        Ok(())
    }

    fn prepare_extension(
        &self,
        file_id: FileId,
        extension_index: usize,
        extension: &'a ExtendDeclaration,
    ) -> Result<()> {
        if !extension.type_args.is_empty() {
            return Err(HIRError::invalid_type(
                self.modules.type_name(extension.target.data),
                InvalidTypeReason::IncorrectUsage,
                extension.span,
            ));
        }
        for method in &extension.methods {
            if !method.type_params.is_empty() {
                return Err(HIRError::invalid_type(
                    method.name,
                    InvalidTypeReason::IncorrectUsage,
                    method.span,
                ));
            }
        }

        self.validate_concrete_type_syntax(file_id, extension.target)?;
        self.validate_concrete_type_syntax(file_id, extension.interface)?;

        let interface_name = match self.modules.get_type(extension.interface.data) {
            Type::Plain(identifier) if identifier.generic.is_empty() => identifier.identifier,
            _ => {
                return Err(HIRError::invalid_type(
                    self.modules.type_name(extension.interface.data),
                    InvalidTypeReason::IncorrectUsage,
                    extension.interface.span,
                ));
            }
        };
        let Some((interface_owner, interface_index)) =
            self.lowerer.lookup.find_interface(interface_name, file_id)
        else {
            return Err(HIRError::invalid_type(
                interface_name,
                InvalidTypeReason::IncorrectUsage,
                extension.interface.span,
            ));
        };
        let interface_id = PoolId::new(interface_index as u32);

        let target = self
            .lowerer
            .lower_type(self, file_id, extension.target, &TypeContext::EMPTY)?
            .term;
        let interface_term = self
            .lowerer
            .lower_type(self, file_id, extension.interface, &TypeContext::EMPTY)?
            .term;
        let interface = self
            .modules
            .get_entry(interface_owner)
            .interfaces()
            .get(interface_id);

        self.validate_extension_methods(
            file_id,
            extension,
            interface_owner,
            &interface.methods,
            target,
        )?;

        let mut methods = Vec::with_capacity(extension.methods.len());
        for method in &extension.methods {
            let declaration_name = self.hir.intern_name(&format!(
                "__interface_impl_{}_{}_{}",
                file_id.as_raw(),
                extension_index,
                self.hir.get_name(method.name)
            ));
            let declaration_id = self.insert_method_declaration(
                file_id,
                method,
                target,
                VisibilityModifier::Public,
                false,
                declaration_name,
                false,
            )?;
            methods.push((method.name, declaration_id));
        }

        let declaration = HirExtendDeclaration {
            target,
            interface: interface_term,
            methods,
        };
        let id = {
            let file = self.hir.store.get_or_create_file(file_id);
            let local = file
                .declarations
                .declarations
                .extensions
                .insert(declaration);
            DeclarationId::new(file_id, local)
        };
        self.hir.types.methods.create_extension(target, id);
        Ok(())
    }

    fn validate_extension_methods(
        &self,
        extension_file: FileId,
        extension: &ExtendDeclaration,
        interface_file: FileId,
        interface_methods: &[slynx_parser::FuncDeclaration],
        target: TermId,
    ) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for method in &extension.methods {
            if !seen.insert(method.name) {
                return Err(HIRError::already_defined(method.name, method.span));
            }
        }

        for required in interface_methods {
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

    fn validate_concrete_type_syntax(
        &self,
        file_id: FileId,
        ty: Spanned<DedupPoolId<Type>>,
    ) -> Result<()> {
        match self.modules.get_type(ty.data) {
            Type::Plain(identifier) => {
                let Some(ASTType { owner, content }) = self
                    .lowerer
                    .lookup
                    .find_type(file_id, identifier.identifier)
                else {
                    return Err(HIRError::type_unrecognized(identifier.identifier, ty.span));
                };
                let generic_count = match content {
                    ASTTypeKind::Struct(id) => self
                        .modules
                        .get_entry(owner)
                        .object()
                        .get(id)
                        .type_params
                        .len(),
                    ASTTypeKind::Component(id) => self
                        .modules
                        .get_entry(owner)
                        .component()
                        .get(id)
                        .type_params
                        .len(),
                    ASTTypeKind::Alias(id) => self
                        .modules
                        .get_entry(owner)
                        .alias()
                        .get(id)
                        .type_params
                        .len(),
                    ASTTypeKind::Enum(id) => self
                        .modules
                        .get_entry(owner)
                        .enums()
                        .get(id)
                        .type_params
                        .len(),
                    ASTTypeKind::Interface(id) => self
                        .modules
                        .get_entry(owner)
                        .interfaces()
                        .get(id)
                        .type_args
                        .len(),
                    ASTTypeKind::Builtin(_) => 0,
                };
                if generic_count != identifier.generic.len() {
                    let reason = if identifier.generic.len() < generic_count {
                        InvalidTypeReason::MissingGeneric
                    } else {
                        InvalidTypeReason::IncorrectUsage
                    };
                    return Err(HIRError::invalid_type(
                        identifier.identifier,
                        reason,
                        ty.span,
                    ));
                }
                for generic in &identifier.generic {
                    self.validate_concrete_type_syntax(file_id, *generic)?;
                }
            }
            Type::Array(element, _) | Type::Vector(element) => {
                self.validate_concrete_type_syntax(file_id, ty.span.make_spanned(*element))?;
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.validate_concrete_type_syntax(file_id, ty.span.make_spanned(*inner))?;
            }
        }
        Ok(())
    }

    pub(crate) fn resolve_interface_method_for_concrete(
        &self,
        requester: FileId,
        ty: TermId,
        method_name: SymbolPointer,
        span: Span,
    ) -> Result<Option<DeclarationId<HirFunctionDeclaration>>> {
        let reachable_files: std::collections::HashSet<FileId> = self
            .lowerer
            .lookup
            .find_all_in_modules((), requester, &|_, ()| Some(()))
            .into_iter()
            .map(|(file_id, ())| file_id)
            .collect();

        let Some(extensions) = self.hir.types.methods.get_extensions_of(ty) else {
            return Ok(None);
        };
        let mut matched = Vec::new();
        for extension_id in extensions.iter() {
            if !reachable_files.contains(&extension_id.file_id) {
                continue;
            }
            matched.push(*extension_id);
        }

        let mut by_interface = std::collections::HashSet::new();
        for extension_id in &matched {
            let extension = self.hir.get_extension(*extension_id);
            if !by_interface.insert(extension.interface) {
                return Err(HIRError::duplicate_interface_implementation(
                    ty,
                    extension.interface,
                    span,
                ));
            }
        }

        for extension_id in matched {
            let extension = self.hir.get_extension(extension_id);
            for (name, method_id) in &extension.methods {
                if *name == method_name {
                    return Ok(Some(*method_id));
                }
            }
        }

        Ok(None)
    }
}
