use common::{
    Span,
    pool::{DedupPoolId, PoolId},
};
use slynx_hir::{
    EnumVariantType, HIRError, HirEnumDeclaration, HirType, Result, SlynxHir,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};

use crate::{
    Monomorphizer,
    types::{MonomorphizationKey, Substitution, mangle_name, substitute_type},
};
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
        let Some((template_file, template_local)) = self.find_enum_declaration_by_name(hir, name)
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
        if template_generics.len() != args.len() {
            return Err(HIRError::generic_arity_mismatch(
                name,
                template_generics.len(),
                args.len(),
                span,
            ));
        }

        let key: MonomorphizationKey = (template_any, args.clone().into());
        if let Some(cached) = self.cache.get(&key) {
            let AnyLocalDeclarationId::Enum(local_id) = cached.local_id else {
                unreachable!("A monomorphized enum target must be an enum")
            };
            return Ok(hir.get_file(cached.file_id).declarations.declarations.enums[local_id].ty);
        }
        if self.in_progress.contains(&key) {
            return Err(HIRError::cyclic_monomorphization(name, args, span));
        }
        self.in_progress.insert(key.clone());

        let subst = Substitution::new(&args);
        let type_variants = {
            enum_view
                .variants()
                .into_iter()
                .map(|variant| {
                    let payload = variant
                        .payload
                        .iter()
                        .map(|ty| {
                            let ty = substitute_type(hir, *ty, &subst)?;
                            self.resolve_expression_type(hir, ty, span)
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

        let mangled_name = mangle_name(hir, name, &args);
        let mangled_symbol = hir.intern_name(&mangled_name);
        let specialized_ty = hir.create_enum_type(mangled_symbol, type_variants);

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
        let specialized = AnyDeclarationId::new(
            template_file,
            AnyLocalDeclarationId::Enum(specialized_local),
        );

        self.cache.insert(key.clone(), specialized);
        self.in_progress.remove(&key);
        self.dead_code.insert(template_any);

        Ok(specialized_ty)
    }
    ///Finds the `HirObjectDeclaration` with the given `name` in any file.
    fn find_enum_declaration_by_name(
        &self,
        hir: &SlynxHir,
        name: slynx_hir::SymbolPointer,
    ) -> Option<(module_loader::FileId, PoolId<HirEnumDeclaration>)> {
        for file in hir.files.iter() {
            for (id, declaration) in file.declarations.declarations.enums.iter().with_ids() {
                if declaration.name == name {
                    return Some((file.file, id));
                }
            }
        }
        None
    }
    ///Neutralizes every generic object template and marks it as dead.
    pub(crate) fn neutralize_generic_enums(
        &mut self,
        hir: &SlynxHir,
        files: &[module_loader::FileId],
        void_ty: DedupPoolId<HirType>,
    ) {
        for file_id in files {
            let generic_ids: Vec<PoolId<HirEnumDeclaration>> = {
                let file = hir.get_file(*file_id);
                file.declarations
                    .declarations
                    .enums
                    .iter()
                    .with_ids()
                    .filter(|(_, declaration)| !declaration.generics.is_empty())
                    .map(|(id, _)| id)
                    .collect()
            };

            for local_id in generic_ids {
                let mut file = hir.get_file_mut(*file_id);
                file.declarations.declarations.enums.get_mut(local_id).ty = void_ty;
                self.dead_code.insert(AnyDeclarationId::new(
                    *file_id,
                    AnyLocalDeclarationId::Enum(local_id),
                ));
            }
        }
    }
}
