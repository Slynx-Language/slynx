//! Struct (object) monomorphization.
//!
//! A generic object template (`object Option<T> { value: T }`) is specialized
//! by [`resolve_object_target`](Monomorphizer::resolve_object_target): for one
//! concrete type-argument list it creates (or retrieves from the cache) a new
//! struct type with substituted field types plus a mangled, non-generic
//! `HirObjectDeclaration`, so codegen hoists a distinct IR struct for it.
//!
//! Generic struct methods are not specialized: the specialized struct is
//! created with an empty method table (see the extension guide).
//!
//! This module also owns the neutralization of the object templates that
//! survive the pass
//! ([`neutralize_generic_objects`](Monomorphizer::neutralize_generic_objects)).

use common::Span;
use module_loader::FileId;
use slynx_hir::{
    DeclarationId, HIRError, HirObjectDeclaration, Result, SlynxHir, SymbolPointer, Visible,
    term::{TermId, TermNode},
};

use crate::{
    Monomorphizer,
    specialization::SpecializationDescriptor,
    types::{MonomorphizationKey, Substitution, substitute_type},
};

impl Monomorphizer {
    ///Given the `HirType::Reference` type of a generic object usage such as
    ///`Option<int>`, returns the specialized struct type, generating a mangled
    ///`HirObjectDeclaration` on first use and deduplicating identical
    ///instantiations afterwards.
    ///
    ///The request is handed to the shared
    ///[`specialize`](Monomorphizer::specialize) skeleton as a
    ///[`SpecializationDescriptor`]; the skeleton reads the template's name and
    ///generic arity itself, and the resulting typed id is narrowed to the
    ///specialization's struct type for the caller.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR being monomorphized; the declaration is inserted into
    ///  the template's file so codegen hoists it next to its template.
    ///* `ty` — the concrete type application (`Option<int>`) to specialize.
    ///* `span` — the use-site span, reported on arity and cycle errors.
    ///
    ///# Returns
    ///
    ///The struct type of the concrete copy — freshly generated or served from
    ///the cache.
    pub(crate) fn resolve_object_target(
        &mut self,
        hir: &SlynxHir,
        ty: TermId,
        span: Span,
    ) -> Result<TermId> {
        let ty_view = hir.view(ty);
        let TermNode::Apply { target, args } = ty_view.raw().node() else {
            unreachable!("resolve_object_target requires a Reference type")
        };

        let rf_view = hir.view(*target);
        let deref = rf_view.dereference();
        let struct_view = deref.is_struct().ok_or_else(|| {
            HIRError::generic_arity_mismatch(
                hir.intern_name(&deref.pretty_name()),
                0,
                args.iter().filter(|slot| !slot.is_null()).count(),
                span,
            )
        })?;
        let name = struct_view.name();

        let Some(template) = self.find_declaration_by_name::<HirObjectDeclaration>(hir, name)
        else {
            unreachable!("Every generic object type must have a HirObjectDeclaration")
        };
        let owner = template.owner;

        let (visibility, external) = {
            let file = hir.get_file(owner);
            let declaration = &file.declarations.declarations.objects[template.term];
            (declaration.visibility, declaration.external)
        };

        let args: Vec<TermId> = args
            .iter()
            .copied()
            .filter(|slot| !slot.is_null())
            .collect();

        let specialized = self.specialize(
            hir,
            SpecializationDescriptor {
                template,
                args,
                span,
                build: |monomorphizer: &mut Monomorphizer,
                        hir: &SlynxHir,
                        subst: &Substitution,
                        mangled_symbol: SymbolPointer,
                        _: &MonomorphizationKey| {
                    let fields = struct_view
                        .fields()
                        .iter()
                        .map(|field| {
                            let new_ty = monomorphizer.resolve_expression_type(
                                hir,
                                substitute_type(hir, field.ty, subst)?,
                                span,
                            )?;
                            Ok(Visible::new(field.visibility, (field.name, new_ty)))
                        })
                        .collect::<Result<Vec<_>>>()?;

                    let specialized_ty =
                        hir.types
                            .create_struct_type(mangled_symbol, fields, Vec::new());

                    let specialized_local = {
                        let file = hir.get_file_mut(owner);
                        file.declarations
                            .declarations
                            .objects
                            .insert(HirObjectDeclaration {
                                name: mangled_symbol,
                                generics: Vec::new(),
                                ty: specialized_ty,
                                visibility,
                                external,
                                attributes: Vec::new(),
                            })
                    };

                    Ok(DeclarationId::new(owner, specialized_local))
                },
            },
        )?;

        let file = hir.get_file(specialized.owner);
        Ok(file.declarations.declarations.objects[specialized.term].ty)
    }

    ///Neutralizes every generic *object* template: empties its method table
    ///and retypes it to `void_ty`, then records it as dead code.
    ///
    ///After this runs no object declaration in the HIR carries a
    ///`GenericParam`-typed signature, so codegen never sees one. Templates
    ///that were instantiated during the pass are neutralized too — their
    ///concrete specializations exist alongside them and are the declarations
    ///codegen emits.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose object templates are neutralized.
    ///* `files` — the files to scan for generic templates.
    ///* `void_ty` — the concrete, non-generic type every neutralized template
    ///  is retyped to.
    pub(crate) fn neutralize_generic_objects(
        &mut self,
        hir: &SlynxHir,
        files: &[FileId],
        void_ty: TermId,
    ) {
        for template in self.generic_templates::<HirObjectDeclaration>(hir, files) {
            let mut file = hir.get_file_mut(template.owner);
            file.declarations
                .declarations
                .objects
                .get_mut(template.term)
                .ty = void_ty;
            self.mark_dead(template);
        }
    }
}
