use common::{Span, pool::DedupPoolId};
use slynx_hir::{
    EnumVariantType, HIRError, HirEnumDeclaration, HirType, Result, SlynxHir,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};

use crate::{Monomorphizer, types::substitute_type};
impl Monomorphizer {
    pub fn resolve_enum_target(
        &mut self,
        hir: &SlynxHir,
        ty: DedupPoolId<HirType>,
        span: Span,
    ) -> Result<DedupPoolId<HirType>> {
        let view = hir.view(ty);
        let HirType::Reference { rf, generics } = view.raw() else {
            return Err(HIRError::not_an_enum(ty, span));
        };
        let reference_view = hir.view(*rf);
        let deref = reference_view.dereference();
        let Some(enum_view) = deref.is_enum() else {
            return Err(HIRError::not_an_enum(*rf, span));
        };
        let name = enum_view.name();
        let Some((template_file, template_local)) =
            self.find_declaration_by_name(hir, name, |pool| &pool.enums, |d| d.name)
        else {
            unreachable!("Every generic enum type must have a HirEnumDeclaration")
        };
        let template_any =
            AnyDeclarationId::new(template_file, AnyLocalDeclarationId::Enum(template_local));

        let (template_generics, visibility, decl_variants) = {
            let file = hir.get_file(template_file);
            let declaration = &file.declarations.declarations.enums[template_local];
            (
                declaration.generics.clone(),
                declaration.visibility,
                declaration.variants.clone(),
            )
        };

        let args: Vec<DedupPoolId<HirType>> = generics
            .iter()
            .copied()
            .filter(|slot| !slot.is_null())
            .collect();

        self.specialize(
            hir,
            name,
            template_any,
            template_generics.len(),
            args,
            span,
            |hir, cached| {
                let AnyLocalDeclarationId::Enum(local_id) = cached.local_id else {
                    unreachable!("A monomorphized enum target must be an enum")
                };
                hir.get_file(cached.file_id).declarations.declarations.enums[local_id].ty
            },
            |monomorphizer, hir, subst, mangled_symbol, _| {
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

                let specialized_ty = hir.types.create_enum_type(mangled_symbol, type_variants);

                let specialized_local = {
                    let file = hir.get_file_mut(template_file);
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

                Ok((
                    AnyDeclarationId::new(
                        template_file,
                        AnyLocalDeclarationId::Enum(specialized_local),
                    ),
                    specialized_ty,
                ))
            },
        )
    }
}
