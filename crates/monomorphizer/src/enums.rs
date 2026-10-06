//! Enum monomorphization.
//!
//! A generic enum template (`enum Option<T> { None, Some(T) }`) is specialized
//! by [`resolve_enum_target`](Monomorphizer::resolve_enum_target): for one
//! concrete type-argument list it creates (or retrieves from the cache) a new
//! enum type with substituted payload types plus a mangled, non-generic
//! `HirEnumDeclaration`, so codegen hoists a distinct IR enum for it.
//!
//! This module also owns the neutralization of the enum templates that
//! survive the pass
//! ([`neutralize_generic_enums`](Monomorphizer::neutralize_generic_enums)).

use common::Span;
use module_loader::FileId;
use slynx_hir::{
    DeclarationId, EnumVariantType, HIRError, HirEnumDeclaration, Result, SlynxHir,
    SymbolPointer,
    term::{TermId, TermNode},
};

use crate::{
    Monomorphizer,
    specialization::SpecializationDescriptor,
    types::{MonomorphizationKey, Substitution, substitute_type},
};

impl Monomorphizer {
    ///Given the type of a generic enum usage such as `Option<int>`, returns
    ///the specialized enum type, generating a mangled `HirEnumDeclaration` on
    ///first use and deduplicating identical instantiations afterwards.
    ///
    ///The request is handed to the shared
    ///[`specialize`](Monomorphizer::specialize) skeleton as a
    ///[`SpecializationDescriptor`]; the skeleton reads the template's name and
    ///generic arity itself, and the resulting typed id is narrowed to the
    ///specialization's enum type for the caller.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR being monomorphized; the declaration is inserted into
    ///  the template's file so codegen hoists it next to its template.
    ///* `ty` — the concrete type application (`Option<int>`) to specialize.
    ///* `span` — the use-site span, reported when `ty` is not a type
    ///  application, when the target is not an enum, and on arity or cycle
    ///  errors.
    ///
    ///# Returns
    ///
    ///The enum type of the concrete copy — freshly generated or served from
    ///the cache.
    pub(crate) fn resolve_enum_target(
        &mut self,
        hir: &SlynxHir,
        ty: TermId,
        span: Span,
    ) -> Result<TermId> {
        let view = hir.view(ty);
        let TermNode::Apply { target, args } = view.raw().node() else {
            return Err(HIRError::not_an_enum(ty, span));
        };
        let reference_view = hir.view(*target);
        let deref = reference_view.dereference();
        let Some(enum_view) = deref.is_enum() else {
            return Err(HIRError::not_an_enum(*target, span));
        };
        let name = enum_view.name();

        let Some(template) = self.find_declaration_by_name::<HirEnumDeclaration>(hir, name) else {
            unreachable!("Every generic enum type must have a HirEnumDeclaration")
        };
        let owner = template.owner;

        let (visibility, decl_variants) = {
            let file = hir.get_file(owner);
            let declaration = &file.declarations.declarations.enums[template.term];
            (declaration.visibility, declaration.variants.clone())
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
                    let type_variants = {
                        enum_view
                            .variants()
                            .iter()
                            .map(|variant| {
                                let payload = variant
                                    .payload
                                    .iter()
                                    .map(|ty| {
                                        let ty = substitute_type(hir, *ty, subst)?;
                                        monomorphizer.resolve_expression_type(hir, ty, span)
                                    })
                                    .collect::<Result<Vec<_>>>()?;
                                Ok(EnumVariantType {
                                    name: variant.name,
                                    discriminant: variant.discriminant,
                                    payload,
                                })
                            })
                            .collect::<Result<Vec<_>>>()?
                    };

                    let specialized_ty =
                        hir.types.create_enum_type(mangled_symbol, type_variants);

                    let specialized_local = {
                        let file = hir.get_file_mut(owner);
                        file.declarations
                            .declarations
                            .enums
                            .insert(HirEnumDeclaration {
                                name: mangled_symbol,
                                generics: Vec::new(),
                                ty: specialized_ty,
                                visibility,
                                variants: decl_variants,
                                attributes: Vec::new(),
                            })
                    };

                    Ok(DeclarationId::new(owner, specialized_local))
                },
            },
        )?;

        let file = hir.get_file(specialized.owner);
        Ok(file.declarations.declarations.enums[specialized.term].ty)
    }

    ///Neutralizes every generic *enum* template: empties its variant list and
    ///retypes it to `void_ty`, then records it as dead code.
    ///
    ///After this runs no enum declaration in the HIR carries a
    ///`GenericParam`-typed signature, so codegen never sees one. Templates
    ///that were instantiated during the pass are neutralized too — their
    ///concrete specializations exist alongside them and are the declarations
    ///codegen emits.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose enum templates are neutralized.
    ///* `files` — the files to scan for generic templates.
    ///* `void_ty` — the concrete, non-generic type every neutralized template
    ///  is retyped to.
    pub(crate) fn neutralize_generic_enums(
        &mut self,
        hir: &SlynxHir,
        files: &[FileId],
        void_ty: TermId,
    ) {
        for template in self.generic_templates::<HirEnumDeclaration>(hir, files) {
            let mut file = hir.get_file_mut(template.owner);
            let declaration = file
                .declarations
                .declarations
                .enums
                .get_mut(template.term);
            declaration.variants = Vec::new();
            declaration.ty = void_ty;
            self.mark_dead(template);
        }
    }
}
