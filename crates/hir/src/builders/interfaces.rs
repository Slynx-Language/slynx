use common::{
    Spanned, VisibilityModifier,
    pool::{DedupPoolId, PoolId},
};
use module_loader::{ASTType, ASTTypeKind, FileId};
use slynx_parser::{ASTFunction, ExtendDeclaration, InterfaceDeclaration, Type, TypeContext};

use crate::{
    DeclarationId, GenericParameter, HIRError, HirExtendDeclaration, HirFunctionDeclaration,
    HirQueueBuilder, InterfaceType, Owned, Result, SymbolPointer,
    builders::lowering::lookup::FindExtensionsWithMethodDescriptor,
    context::{HirSymbol, InterfaceMethodSignature},
    error::MissingFeature,
    term::TermId,
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
                if !interface.generics.type_params.is_empty() {
                    return Err(HIRError::unimplemented(
                        MissingFeature::GenericInterfaces,
                        interface.span,
                    ));
                }
                if let Some(method) = interface
                    .methods
                    .iter()
                    .find(|method| !method.generics.type_params.is_empty())
                {
                    return Err(HIRError::unimplemented(
                        MissingFeature::GenericInterfaces,
                        method.span,
                    ));
                }
            }

            for extension in module.extensions().iter() {
                if !extension.generics.type_params.is_empty() {
                    return Err(HIRError::unimplemented(
                        MissingFeature::GenericInterfaces,
                        extension.span,
                    ));
                }
                if let Some(method) = extension
                    .methods
                    .iter()
                    .find(|method| !method.generics.type_params.is_empty())
                {
                    return Err(HIRError::unimplemented(
                        MissingFeature::GenericInterfaces,
                        method.span,
                    ));
                }
                self.assert_concrete_type_generic_count(owner, extension.target)?;
                for interface in extension.generics.interface_implementations.iter() {
                    if let Type::Plain(identifier) = self.modules.get_type(interface.data) {
                        if !identifier.generic.is_empty() {
                            return Err(HIRError::unimplemented(
                                MissingFeature::GenericInterfaces,
                                interface.span,
                            ));
                        }
                    } else {
                        return Err(HIRError::unimplemented(
                            MissingFeature::GenericInterfaces,
                            interface.span,
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    ///Materializes the signature of an interface method as a bodyless
    ///[`HirFunctionDeclaration`] keyed by `Self = Var(0)`, and registers it
    ///against the declaring interface.
    ///
    ///A method call on a bounded generic parameter (`func f<T>(x: T) where T: I`
    ///calling `x.m()`) has no concrete target while the HIR is being built. It
    ///resolves to this declaration instead, so the existing call building path
    ///works unchanged, and is discharged by the monomorphizer once the
    ///receiver's concrete type is known.
    ///
    ///Idempotent: every later request for the same method returns the very same
    ///declaration.
    pub(crate) fn insert_interface_method_signature(
        &self,
        owner: FileId,
        interface: DedupPoolId<InterfaceType>,
        interface_name: SymbolPointer,
        method: &slynx_parser::FuncDeclaration,
        signature: TermId,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        let name = self.hir.intern_name(&format!(
            "{}_interface_{}",
            self.hir.get_name(method.name),
            self.hir.get_name(interface_name),
        ));
        let declaration = self.hir.symbols_registry.get_or_insert_function(
            HirSymbol::new(owner, name),
            || {
                Ok(self.hir.store.get_or_create_file(owner).create_function(
                    HirFunctionDeclaration {
                        name,
                        // `Self` is generic parameter 0 so the receiver supplied
                        // at the call site substitutes it.
                        generics: vec![GenericParameter {
                            name: self.hir.intern_name("Self"),
                            bounds: Vec::new(),
                        }],
                        args: Default::default(),
                        ty: signature,
                        statements: Vec::new(),
                        visibility: VisibilityModifier::Public,
                        external: false,
                        attributes: Vec::new(),
                        span: method.span,
                    },
                ))
            },
        )?;
        self.hir
            .types
            .create_interface_method(InterfaceMethodSignature {
                interface,
                name: method.name,
                declaration,
            });
        Ok(declaration)
    }

    ///Materializes every reachable extension that declares a method named
    ///`name`, keyed by the concrete type it extends.
    ///
    ///A deferred interface call cannot pick its implementation while the HIR is
    ///built — the receiver is still a generic parameter. Materializing every
    ///candidate implementation up front is what lets the monomorphizer pick one
    ///later, once the concrete self type is known.
    pub(crate) fn materialize_extensions_declaring_method(
        &self,
        requester: FileId,
        name: SymbolPointer,
    ) -> Result<()> {
        for extension in
            self.lowerer
                .lookup
                .find_extensions_with_method(FindExtensionsWithMethodDescriptor {
                    requester,
                    method_name: name,
                })
        {
            let declaration = self
                .modules
                .get_entry(extension.owner)
                .extensions()
                .get(extension.term);
            self.assert_concrete_type_generic_count(extension.owner, declaration.target)?;
            let target = self
                .lowerer
                .lower_type(
                    self,
                    extension.owner,
                    declaration.target,
                    &TypeContext::EMPTY,
                )?
                .term;
            self.materialize_interface_implementation(InterfaceImplementationDescriptor {
                extension,
                target,
            })?;
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

        if !extension.generics.type_params.is_empty() {
            return Err(HIRError::unimplemented(
                MissingFeature::GenericInterfaces,
                extension.span,
            ));
        }
        for method in &extension.methods {
            if !method.generics.type_params.is_empty() {
                return Err(HIRError::unimplemented(
                    MissingFeature::GenericInterfaces,
                    method.span,
                ));
            }
        }

        self.assert_concrete_type_generic_count(descriptor.extension.owner, extension.target)?;

        let (interface_ids, interface_methods) = {
            let mut interfaces = Vec::new();
            let mut interface_methods = std::collections::HashMap::new(); //hashmap name->interface_id
            for interface in extension.generics.interface_implementations() {
                self.assert_concrete_type_generic_count(descriptor.extension.owner, *interface)?;
                let interface_name = match self.modules.get_type(interface.data) {
                    Type::Plain(identifier) if identifier.generic.is_empty() => {
                        identifier.identifier
                    }
                    _ => {
                        return Err(HIRError::unimplemented(
                            MissingFeature::GenericInterfaces,
                            interface.span,
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
                    return Err(HIRError::type_unrecognized(interface_name, interface.span));
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
                    .materialize_interface_definition(
                        self,
                        ASTType {
                            owner: interface_owner,
                            content: ASTTypeKind::Interface(interface_id),
                        },
                        interface.span,
                    )?
                    .term;

                if let Some(existing) = self.hir.types.methods.get_extensions_of(descriptor.target) {
                    for existing_id in existing.value().iter() {
                        let existing_impl = self.hir.get_extension(*existing_id);
                        if existing_impl.interfaces.iter().any(|existing_interface| {
                            *existing_interface == interface_term
                        }) {
                            return Err(HIRError::duplicate_interface_implementation(
                                descriptor.target,
                                interface_term,
                                interface.span,
                            ));
                        }
                    }
                }

                interfaces.push(interface_term);
                for method in interface_declaration.methods.iter() {
                    interface_methods.insert(method.name, interface_term);
                }
            }
            (interfaces, interface_methods)
        };
        let mut methods = Vec::new();

        for method in &extension.methods {
            let declaration_name = self.hir.intern_name(&format!(
                "{}_interface_{}_{}",
                self.hir.view(descriptor.target).internal_name(),
                self.hir
                    .view(
                        *interface_methods
                            .get(&method.name)
                            .expect("Expected interface to be checked to contain method",)
                    )
                    .internal_name(),
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
            generics: Vec::new(),
            target: descriptor.target,
            interfaces: interface_ids,
            methods,
            attributes: Vec::new(),
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

    ///Makes validations to see if interface methods are implemented correctly. Thus with no repetition and if all the methods are properly implemented
    fn validate_extension_methods(
        &self,
        extension_file: FileId,
        extension: &ExtendDeclaration,
        interface_file: FileId,
        interface: &InterfaceDeclaration,
        target: TermId,
    ) -> Result<()> {
        {
            // Exist only because the codebase does not support `extend<T,L> MyType<T>: Interface<L> {}` yet. After so, all this can be removed
            if !interface.generics.type_params.is_empty() {
                return Err(HIRError::unimplemented(
                    MissingFeature::GenericInterfaces,
                    interface.span,
                ));
            }
            for method in &interface.methods {
                if !method.generics.type_params.is_empty() {
                    return Err(HIRError::unimplemented(
                        MissingFeature::GenericInterfaces,
                        method.span,
                    ));
                }
            }
        }

        let mut seen = std::collections::HashMap::new();
        for method in &extension.methods {
            if seen.insert(method.name, method).is_some() {
                return Err(HIRError::already_defined(method.name, method.span));
            }
        }

        for required in &interface.methods {
            if let Some(implementation) = seen.get(&required.name) {
                let expected = self.method_signature(interface_file, required, target)?;
                let actual = self.method_signature(extension_file, *implementation, target)?;
                if expected != actual {
                    return Err(HIRError::unexpected_type(
                        actual,
                        expected,
                        implementation.span,
                    ));
                }
            } else {
                return Err(HIRError::missing_interface_method(
                    required.name,
                    extension.span,
                ));
            }
        }
        Ok(())
    }

    ///Gets the signature of the given `method` function.
    fn method_signature<T: ASTFunction>(
        &self,
        owner: FileId,
        method: &T,
        self_type: TermId,
    ) -> Result<TermId> {
        let context = TypeContext::new(method.generics().type_params());
        let lower_type = |ty: Spanned<DedupPoolId<Type>>| {
            self.lowerer
                .lower_type_with_self(self, owner, ty, &context, self_type)
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
