use common::{Span, VisibilityModifier, pool::PoolId};
use module_loader::ASTTypeKind;
use slynx_parser::{ExtendDeclaration, InterfaceDeclaration, ObjectMethod, TypeContext};

use crate::{
    DeclarationId, DescriptorId, ExpressionBuilder, GenericParameter, HIRError,
    HirExtendDeclaration, HirFunctionDeclaration, HirQueueBuilder, Owned, Result, SymbolPointer,
    builders::{
        interfaces::InterfaceImplementationDescriptor,
        lowering::lookup::FindExtensionsWithMethodDescriptor,
    },
    error::MissingFeature,
    id::OwnerId,
    term::{TermId, TermNode},
};

pub enum TypeMethodResolution {
    ///Represents a method implemented via an interface extension.
    Interface {
        ///The extension declaration that implements the method.
        extension: DeclarationId<HirExtendDeclaration>,
        ///The method being implemented.
        method: DeclarationId<HirFunctionDeclaration>,
    },
    Inherent(DeclarationId<HirFunctionDeclaration>),
    Static(DeclarationId<HirFunctionDeclaration>),
}

pub struct FindInterfaceMethodDescriptor {
    ///The type to search for the method on.
    pub ty: Owned<TermId>,
    ///The name of the method to be found
    pub name: SymbolPointer,
    ///The span of the code that is requesting this
    pub span: Span,
}

pub struct FindInherentMethodDescriptor {
    ///The type to search for the method on.
    pub ty: Owned<TermId>,
    ///The name of the method to be found
    pub name: SymbolPointer,
    pub span: Span,
}

pub struct FindBoundedMethodDescriptor {
    ///The index of the generic parameter the receiver has as its type.
    pub parameter: u8,
    ///The name of the method to be found
    pub name: SymbolPointer,
}

///Finds all the interfaces the given `ty` implements across all the given `candidates` extensions.
///The given `ty` is the type we are searching for and the `candidates` are extensions that may implement interfaces for it.
///
///For example, when finding '5.abs()' the given `ty` will be Int, or any other number type, and the `candidates` are any extensions desired.
///What this will do is to filter these extensions and find the ones whose target is the given `ty`.
///Thus, for example `extend Int: Math {func abs() {}}` would be a candidate for implementing `abs()` for `Int`,
/// and `Math` interface would be included on `FindInterfaceImplementationsResult.interfaces`.
pub struct FindInterfaceImplementationsDescriptor<'a> {
    ///The type to search for implementations of.
    pub ty: Owned<TermId>,
    ///The id of the extensions that are candidates for implementing this interface. Thus, if a given `ty` T, the candidates are all extensions such as `extend T: Interface`
    pub candidates: &'a [Owned<PoolId<ExtendDeclaration>>],
    pub span: Span,
}

pub struct FindInterfaceImplementationsResult {
    pub interfaces: Vec<Owned<PoolId<InterfaceDeclaration>>>,
    pub extension: Owned<PoolId<ExtendDeclaration>>,
    pub target: TermId,
}

impl ExpressionBuilder {
    pub fn find_inherent_method_of(
        &self,
        queue: &HirQueueBuilder,
        descriptor: FindInherentMethodDescriptor,
    ) -> Result<Option<DeclarationId<HirFunctionDeclaration>>> {
        let Owned { owner, term } = descriptor.ty;
        let descriptor_name = match queue.hir.view(term).raw().node() {
            //only descriptor types mau have inherent methods.
            TermNode::Data(DescriptorId::Struct(id)) => queue.hir.types.get_struct_name(*id),
            TermNode::Data(DescriptorId::Enum(id)) => queue.hir.types.get_enum_name(*id),
            TermNode::Data(DescriptorId::Component(id)) => queue.hir.types.get_component_name(*id),
            _ => return Ok(None),
        };
        let Some(asttype) = queue.lowerer.lookup.find_type(owner, descriptor_name) else {
            return Err(HIRError::type_unrecognized(
                descriptor_name,
                descriptor.span,
            ));
        };
        let (methods, external, visibility): (&[ObjectMethod], bool, VisibilityModifier) =
            match asttype.content {
                ASTTypeKind::Struct(strukt) => {
                    let object = queue.modules.get_entry(asttype.owner).object().get(strukt);
                    (&object.methods, object.external, object.visibility)
                }
                ASTTypeKind::Component(component) => {
                    let component = queue
                        .modules
                        .get_entry(asttype.owner)
                        .component()
                        .get(component);
                    (&component.methods, false, component.visibility)
                }
                ASTTypeKind::Enum(enum_) => {
                    let enum_ = queue.modules.get_entry(asttype.owner).enums().get(enum_);
                    (&enum_.methods, false, enum_.visibility)
                }
                _ => (&[][..], false, VisibilityModifier::Public),
            };

        let Some(method) = methods
            .iter()
            .find(|method| method.method_name == descriptor.name)
        else {
            return Ok(None);
        };

        let declaration_name = queue.hir.intern_name(&format!(
            "{}_{}",
            queue.hir.get_name(descriptor.name),
            queue.hir.get_name(descriptor_name)
        ));
        queue
            .insert_method_declaration(
                asttype.owner,
                method,
                descriptor.ty.term,
                visibility,
                external,
                declaration_name,
                true,
            )
            .map(Some)
    }

    pub fn find_interface_implementations_of(
        &self,
        queue: &HirQueueBuilder,
        descriptor: FindInterfaceImplementationsDescriptor,
    ) -> Result<Vec<FindInterfaceImplementationsResult>> {
        let mut matching = Vec::new();

        for extension_id in descriptor.candidates {
            let extension = queue
                .modules
                .get_entry(extension_id.owner)
                .extensions()
                .get(extension_id.term);

            queue.assert_concrete_type_generic_count(extension_id.owner, extension.target)?;
            let target = queue
                .lowerer
                .lower_type(
                    queue,
                    extension_id.owner,
                    extension.target,
                    &TypeContext::EMPTY,
                )?
                .term;
            let matched_target =
                match self.unify_terms(queue, target, descriptor.ty.term, descriptor.span) {
                    Ok(term) => term,
                    Err(_) => continue,
                };
            let target = matched_target;
            let interfaces = {
                let mut out = Vec::new();
                for interface in extension.generics.interface_implementations.iter() {
                    let interface = match queue.modules.get_type(interface.data) {
                        slynx_parser::Type::Plain(identifier)
                            if identifier.generic.is_empty()
                                && let Some(interface) = queue
                                    .lowerer
                                    .lookup
                                    .find_interface(identifier.identifier, extension_id.owner) =>
                        {
                            interface
                        }
                        _ => {
                            return Err(HIRError::unimplemented(
                                MissingFeature::GenericInterfaces,
                                interface.span,
                            ));
                        }
                    };

                    out.push(interface);
                }
                out
            };
            matching.push(FindInterfaceImplementationsResult {
                interfaces,
                extension: *extension_id,
                target,
            });
        }
        Ok(matching)
    }

    pub fn find_interface_method_of(
        &self,
        queue: &HirQueueBuilder,
        descriptor: FindInterfaceMethodDescriptor,
    ) -> Result<Option<TypeMethodResolution>> {
        let candidates =
            queue
                .lowerer
                .lookup
                .find_extensions_with_method(FindExtensionsWithMethodDescriptor {
                    requester: self.file(),
                    method_name: descriptor.name,
                });

        let mut implemented_interfaces = std::collections::HashMap::new();
        let mut resolved_methods = Vec::new();
        for result in self.find_interface_implementations_of(
            queue,
            FindInterfaceImplementationsDescriptor {
                ty: descriptor.ty,
                candidates: &candidates,
                span: descriptor.span,
            },
        )? {
            if let Some(interface_type) = implemented_interfaces.get(&result.extension) {
                return Err(HIRError::duplicate_interface_implementation(
                    descriptor.ty.term,
                    *interface_type,
                    descriptor.span,
                ));
            }
            let extension_id =
                queue.materialize_interface_implementation(InterfaceImplementationDescriptor {
                    extension: result.extension,
                    target: result.target,
                })?;
            let implementation = queue.hir.get_extension(extension_id);
            implemented_interfaces.insert(result.extension, implementation.target);
            if let Some((_, method)) = implementation
                .methods
                .iter()
                .find(|(name, _)| *name == descriptor.name)
            {
                resolved_methods.push(TypeMethodResolution::Interface {
                    extension: extension_id,
                    method: *method,
                });
            }
        }
        Ok(resolved_methods.into_iter().next())
    }

    ///Resolves a method call whose receiver is typed as one of the enclosing
    ///declaration's generic parameters, such as `x` in
    ///`func describe<T>(x: T) where T: Stringifiable -> x.stringify()`.
    ///
    ///The receiver is not concrete yet, so no implementation can be picked here.
    ///The call resolves to the interface method's signature declaration — which
    ///reuses the whole method-call path, receiver included — and the
    ///monomorphizer discharges it once it knows the concrete self type.
    pub fn find_bounded_method_of(
        &self,
        queue: &HirQueueBuilder,
        descriptor: FindBoundedMethodDescriptor,
    ) -> Result<Option<DeclarationId<HirFunctionDeclaration>>> {
        let Some(parameter) = self
            .generics_of(queue)
            .into_iter()
            .nth(descriptor.parameter as usize)
        else {
            return Ok(None);
        };
        for bound in &parameter.bounds {
            let Some(declaration) = queue
                .hir
                .types
                .interface_method_of(bound.raw, descriptor.name)
            else {
                continue;
            };
            // The self type is still unknown, so every reachable implementation
            // of this method has to be materialized now for the monomorphizer
            // to choose from later.
            queue.materialize_extensions_declaring_method(self.file(), descriptor.name)?;
            return Ok(Some(declaration));
        }
        Ok(None)
    }

    ///The generic parameters of the declaration currently being built.
    fn generics_of(&self, queue: &HirQueueBuilder) -> Vec<GenericParameter> {
        match self.target {
            OwnerId::Function(f) => queue.hir.get_function(f).generics.clone(),
            OwnerId::Component(c) => queue.hir.get_component(c).generics.clone(),
        }
    }
}
