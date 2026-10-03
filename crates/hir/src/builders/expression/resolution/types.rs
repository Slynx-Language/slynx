use common::{Span, VisibilityModifier};
use module_loader::ASTTypeKind;
use slynx_parser::{ObjectMethod, TypeContext};

use crate::{
    DeclarationId, DescriptorId, ExpressionBuilder, HIRError, HirExtendDeclaration,
    HirFunctionDeclaration, HirQueueBuilder, Owned, Result, SymbolPointer,
    builders::{
        interfaces::InterfaceImplementationDescriptor,
        lowering::lookup::FindExtensionsWithMethodDescriptor,
    },
    error::InvalidTypeReason,
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
    pub ty: Owned<TermId>,
    pub name: SymbolPointer,
    pub span: Span,
}

pub struct FindInherentMethodDescriptor {
    ///The type to search for the method on.
    pub ty: Owned<TermId>,
    ///The name of the method to be found
    pub name: SymbolPointer,
    pub span: Span,
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

        let mut matching = Vec::new();
        for extension_id in candidates {
            let extension = queue
                .modules
                .get_entry(extension_id.owner)
                .extensions()
                .get(extension_id.term);

            if !extension.type_args.is_empty() {
                return Err(HIRError::invalid_type(
                    queue.modules.type_name(extension.target.data),
                    InvalidTypeReason::Unimplemented,
                    extension.span,
                ));
            }
            if extension
                .methods
                .iter()
                .any(|method| method.name == descriptor.name && !method.type_params.is_empty())
            {
                let method = extension
                    .methods
                    .iter()
                    .find(|method| method.name == descriptor.name)
                    .expect("method was found above");
                return Err(HIRError::invalid_type(
                    method.name,
                    InvalidTypeReason::Unimplemented,
                    method.span,
                ));
            }

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
            if target != descriptor.ty.term {
                continue;
            }

            let interface_name = match queue.modules.get_type(extension.interface.data) {
                slynx_parser::Type::Plain(identifier) if identifier.generic.is_empty() => {
                    identifier.identifier
                }
                _ => {
                    return Err(HIRError::invalid_type(
                        queue.modules.type_name(extension.interface.data),
                        InvalidTypeReason::Unimplemented,
                        extension.interface.span,
                    ));
                }
            };
            let Some(interface) = queue
                .lowerer
                .lookup
                .find_interface(interface_name, extension_id.owner)
            else {
                return Err(HIRError::invalid_type(
                    interface_name,
                    InvalidTypeReason::Unimplemented,
                    extension.interface.span,
                ));
            };
            matching.push((extension_id, target, interface));
        }

        let mut implemented_interfaces = std::collections::HashMap::new();
        let mut resolved_methods = Vec::new();
        for (extension, target, interface) in matching {
            if let Some(interface_type) = implemented_interfaces.get(&interface) {
                return Err(HIRError::duplicate_interface_implementation(
                    descriptor.ty.term,
                    *interface_type,
                    descriptor.span,
                ));
            }
            let extension_id =
                queue.materialize_interface_implementation(InterfaceImplementationDescriptor {
                    extension,
                    target,
                })?;
            let implementation = queue.hir.get_extension(extension_id);
            implemented_interfaces.insert(interface, implementation.interface);
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
}
