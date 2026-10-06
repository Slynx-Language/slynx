//! Function monomorphization.
//!
//! A generic function template (`func identity<T>(x: T): T`) is specialized by
//! [`resolve_function_target`](Monomorphizer::resolve_function_target): for one
//! concrete type-argument list it creates (or retrieves from the cache) a
//! concrete, mangled `HirFunctionDeclaration` whose signature and body have
//! every generic parameter substituted.

use common::Span;
use slynx_hir::{
    DeclarationId, HIRError, HirFunctionDeclaration, Result, SlynxHir,
    context::InterfaceMethodSignature,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
    term::TermId,
};

use crate::{
    Monomorphizer,
    types::{Substitution, contains_generic_param, substitute_type},
};

impl Monomorphizer {
    ///Discharges a deferred interface call: the receiver is no longer a generic
    ///parameter, so the implementation extending its concrete type with the
    ///method the interface declares is selected.
    ///
    ///The implementation is already concrete — an extension method has `Self`
    ///substituted for its target and cannot be generic — so it is returned
    ///as-is rather than specialized again.
    pub(crate) fn resolve_interface_call(
        &mut self,
        hir: &SlynxHir,
        signature: &InterfaceMethodSignature,
        receiver: TermId,
        subst: &Substitution,
        span: Span,
    ) -> Result<AnyDeclarationId> {
        // The receiver is passed by reference when the method takes one, and the
        // extension is keyed by the type being extended.
        let receiver = hir.view(receiver).concrete_type().data();
        let receiver = substitute_type(hir, receiver, subst)?;
        let receiver = if contains_generic_param(hir, receiver) {
            return Err(HIRError::unresolved_interface_call(
                signature.name,
                receiver,
                span,
            ));
        } else {
            self.resolve_expression_type(hir, receiver, span)?
        };
        let method = hir
            .get_extension_method(receiver, signature.name)
            .ok_or_else(|| HIRError::unresolved_interface_call(signature.name, receiver, span))?;
        Ok(AnyDeclarationId::new(
            method.owner,
            AnyLocalDeclarationId::Function(method.term),
        ))
    }

    ///Generates (or retrieves from the cache) the specialization of the generic
    ///`template` function with the given concrete type `args`.
    pub(crate) fn resolve_function_target(
        &mut self,
        hir: &SlynxHir,
        template: DeclarationId<HirFunctionDeclaration>,
        args: Vec<TermId>,
        span: Span,
    ) -> Result<AnyDeclarationId> {
        let template_any = AnyDeclarationId::new(
            template.owner,
            AnyLocalDeclarationId::Function(template.term),
        );

        let (name, generics, fargs, fty, statements, visibility, external, attributes) = {
            let file = hir.get_file(template.owner);
            let declaration = &file.declarations.declarations.functions[template.term];
            (
                declaration.name,
                declaration.generics.clone(),
                declaration.args.clone(),
                declaration.ty,
                declaration.statements.clone(),
                declaration.visibility,
                declaration.external,
                declaration.attributes.clone(),
            )
        };

        self.specialize(
            hir,
            name,
            template_any,
            generics.len(),
            args,
            span,
            |_, cached| cached,
            |monomorphizer, hir, subst, mangled_symbol, key| {
                let specialized_ty = monomorphizer.resolve_expression_type(
                    hir,
                    substitute_type(hir, fty, subst)?,
                    span,
                )?;

                let specialized_local = {
                    let file = hir.get_file_mut(template.owner);
                    file.declarations
                        .declarations
                        .functions
                        .insert(HirFunctionDeclaration {
                            name: mangled_symbol,
                            generics: Vec::new(),
                            args: fargs,
                            ty: specialized_ty,
                            statements: Vec::new(),
                            visibility,
                            external,
                            attributes,
                            span,
                        })
                };
                let specialized = AnyDeclarationId::new(
                    template.owner,
                    AnyLocalDeclarationId::Function(specialized_local),
                );

                // Cache *before* generating the body so recursive instantiations
                // of the same (template, args) resolve to this very declaration.
                monomorphizer.cache.insert(key.clone(), specialized);

                let new_statements = monomorphizer.build_statements(hir, &statements, subst)?;
                {
                    let mut file = hir.get_file_mut(template.owner);
                    file.declarations
                        .declarations
                        .functions
                        .get_mut(specialized_local)
                        .statements = new_statements;
                }

                Ok((specialized, specialized))
            },
        )
    }

    ///Retrieves the return type of the given (already specialized) function
    ///declaration.
    pub(crate) fn function_return_type(
        &self,
        hir: &SlynxHir,
        id: AnyDeclarationId,
    ) -> Result<TermId> {
        let AnyLocalDeclarationId::Function(local_id) = id.term else {
            unreachable!("A monomorphized call target must be a function")
        };
        let file = hir.get_file(id.owner);
        let declaration = &file.declarations.declarations.functions[local_id];
        let view = hir.view(declaration.ty);
        let (_, return_type) = view
            .is_function()
            .expect("Function declaration should have a function type");
        Ok(return_type)
    }
}
