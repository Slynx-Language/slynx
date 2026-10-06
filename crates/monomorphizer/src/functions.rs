//! Function monomorphization.
//!
//! A generic function template (`func identity<T>(x: T): T`) is specialized by
//! [`resolve_function_target`](Monomorphizer::resolve_function_target): for one
//! concrete type-argument list it creates (or retrieves from the cache) a
//! concrete, mangled `HirFunctionDeclaration` whose signature and body have
//! every generic parameter substituted.
//!
//! This module also owns the neutralization of the function templates that
//! survive the pass
//! ([`neutralize_generic_functions`](Monomorphizer::neutralize_generic_functions)).

use common::Span;
use module_loader::FileId;
use slynx_hir::{
    DeclarationId, HIRError, HirFunctionDeclaration, Result, SlynxHir, SymbolPointer,
    TypeDeclaration,
    context::InterfaceMethodSignature,
    term::TermId,
};

use crate::{
    Monomorphizer,
    specialization::SpecializationDescriptor,
    types::{MonomorphizationKey, Substitution, contains_generic_param, substitute_type},
};

impl Monomorphizer {
    ///Discharges a deferred interface call: the receiver is no longer a generic
    ///parameter, so the implementation extending its concrete type with the
    ///method the interface declares is selected.
    ///
    ///The implementation is already concrete — an extension method has `Self`
    ///substituted for its target and cannot be generic — so it is returned
    ///as-is rather than specialized again.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR holding the extensions and the receiver's type.
    ///* `signature` — the interface method signature the call was deferred to.
    ///* `receiver` — the type of the call's first argument, from which the
    ///  extended type is derived.
    ///* `subst` — the substitution of the enclosing instantiation, applied to
    ///  the receiver before its extension is looked up.
    ///* `span` — the call-site span, reported when the receiver still
    ///  mentions a generic parameter or no extension provides the method.
    ///
    ///# Returns
    ///
    ///The typed id of the extension method implementing the call.
    pub(crate) fn resolve_interface_call(
        &mut self,
        hir: &SlynxHir,
        signature: &InterfaceMethodSignature,
        receiver: TermId,
        subst: &Substitution,
        span: Span,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
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
        hir.get_extension_method(receiver, signature.name)
            .ok_or_else(|| HIRError::unresolved_interface_call(signature.name, receiver, span))
    }

    ///Generates (or retrieves from the cache) the specialization of the generic
    ///`template` function with the given concrete type `args`.
    ///
    ///The request is handed to the shared [`specialize`](Monomorphizer::specialize)
    ///skeleton as a [`SpecializationDescriptor`]; this function only reads the
    ///parts of the template the build callback needs (arguments, signature,
    ///body, visibility) up front — the template's name and generic arity are
    ///derived by the skeleton itself.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR being monomorphized; the specialization is inserted
    ///  into `template`'s file so codegen hoists it next to its template.
    ///* `template` — the generic function to copy, as a typed id.
    ///* `args` — the concrete type arguments of this instantiation, in
    ///  parameter order; must match the template's generic arity.
    ///* `span` — the call-site span, reported on arity and cycle errors.
    ///
    ///# Returns
    ///
    ///The typed id of the concrete copy — freshly generated or served from
    ///the cache.
    pub(crate) fn resolve_function_target(
        &mut self,
        hir: &SlynxHir,
        template: DeclarationId<HirFunctionDeclaration>,
        args: Vec<TermId>,
        span: Span,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        let owner = template.owner;
        let (fargs, fty, statements, visibility, external, attributes) = {
            let file = hir.get_file(owner);
            let declaration = &file.declarations.declarations.functions[template.term];
            (
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
            SpecializationDescriptor {
                template,
                args,
                span,
                build: |monomorphizer: &mut Monomorphizer,
                        hir: &SlynxHir,
                        subst: &Substitution,
                        mangled_symbol: SymbolPointer,
                        key: &MonomorphizationKey| {
                    let specialized_ty = monomorphizer.resolve_expression_type(
                        hir,
                        substitute_type(hir, fty, subst)?,
                        span,
                    )?;

                    let specialized_local = {
                        let file = hir.get_file_mut(owner);
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
                    let specialized = DeclarationId::new(owner, specialized_local);

                    // Cache *before* generating the body so recursive instantiations
                    // of the same (template, args) resolve to this very declaration.
                    monomorphizer
                        .cache
                        .insert(key.clone(), HirFunctionDeclaration::as_any_id(specialized));

                    let new_statements = monomorphizer.build_statements(hir, &statements, subst)?;
                    {
                        let mut file = hir.get_file_mut(owner);
                        file.declarations
                            .declarations
                            .functions
                            .get_mut(specialized_local)
                            .statements = new_statements;
                    }

                    Ok(specialized)
                },
            },
        )
    }

    ///Neutralizes every generic *function* template: empties its body and
    ///retypes it to `void_ty`, then records it as dead code.
    ///
    ///After this runs no function declaration in the HIR carries a
    ///`GenericParam`-typed signature, so codegen never sees one. Templates
    ///that were instantiated during the pass are neutralized too — their
    ///concrete specializations exist alongside them and are the declarations
    ///codegen emits.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose function templates are neutralized.
    ///* `files` — the files to scan for generic templates.
    ///* `void_ty` — the concrete, non-generic function type every neutralized
    ///  template is retyped to.
    pub(crate) fn neutralize_generic_functions(
        &mut self,
        hir: &SlynxHir,
        files: &[FileId],
        void_ty: TermId,
    ) {
        for template in self.generic_templates::<HirFunctionDeclaration>(hir, files) {
            let mut file = hir.get_file_mut(template.owner);
            let declaration = file
                .declarations
                .declarations
                .functions
                .get_mut(template.term);
            declaration.statements = Vec::new();
            declaration.ty = void_ty;
            self.mark_dead(template);
        }
    }

    ///Retrieves the return type of the given (already specialized) function
    ///declaration.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR holding the declaration.
    ///* `id` — the typed id of the (specialized) function.
    ///
    ///# Returns
    ///
    ///The right-hand side of the declaration's function type.
    pub(crate) fn function_return_type(
        &self,
        hir: &SlynxHir,
        id: DeclarationId<HirFunctionDeclaration>,
    ) -> Result<TermId> {
        let file = hir.get_file(id.owner);
        let declaration = &file.declarations.declarations.functions[id.term];
        let view = hir.view(declaration.ty);
        let (_, return_type) = view
            .is_function()
            .expect("Function declaration should have a function type");
        Ok(return_type)
    }
}
