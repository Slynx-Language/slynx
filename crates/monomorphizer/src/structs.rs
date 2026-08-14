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

use common::{
    Span,
    pool::{DedupPoolId, PoolId},
};
use slynx_hir::{
    HIRError, HirObjectDeclaration, HirType, Result, SlynxHir, Visible,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};

use crate::{
    Monomorphizer,
    types::{MonomorphizationKey, Substitution, mangle_name, substitute_type},
};

impl Monomorphizer {
    ///Given the `HirType::Reference` type of a generic object usage such as
    ///`Option<int>`, returns the specialized struct type, generating a mangled
    ///`HirObjectDeclaration` on first use and deduplicating identical
    ///instantiations afterwards.
    pub(crate) fn resolve_object_target(
        &mut self,
        hir: &SlynxHir,
        ty: DedupPoolId<HirType>,
        span: Span,
    ) -> Result<DedupPoolId<HirType>> {
        let ty_view = hir.view(ty);
        let HirType::Reference { rf, generics } = ty_view.raw() else {
            unreachable!("resolve_object_target requires a Reference type")
        };

        let ty_view = hir.view(*rf);
        let deref = ty_view.dereference();
        let struct_view = deref.is_struct().ok_or_else(|| {
            HIRError::generic_arity_mismatch(
                hir.intern_name("<non-struct>"),
                0,
                generics.iter().filter(|slot| !slot.is_null()).count(),
                span,
            )
        })?;
        let name = struct_view.name();

        let Some((template_file, template_local)) =
            self.find_object_declaration_by_name(hir, name)
        else {
            unreachable!("Every generic object type must have a HirObjectDeclaration")
        };
        let template_any =
            AnyDeclarationId::new(template_file, AnyLocalDeclarationId::Object(template_local));

        let (template_generics, visibility, external) = {
            let file = hir.get_file(template_file);
            let declaration = &file.declarations.declarations.objects[template_local];
            (
                declaration.generics.clone(),
                declaration.visibility,
                declaration.external,
            )
        };

        let args: Vec<DedupPoolId<HirType>> =
            generics.iter().copied().filter(|slot| !slot.is_null()).collect();
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
            let AnyLocalDeclarationId::Object(local_id) = cached.local_id else {
                unreachable!("A monomorphized object target must be an object")
            };
            return Ok(hir.get_file(cached.file_id).declarations.declarations.objects[local_id].ty);
        }
        if self.in_progress.contains(&key) {
            return Err(HIRError::cyclic_monomorphization(name, args, span));
        }
        self.in_progress.insert(key.clone());

        let subst = Substitution::new(&template_generics, &args);
        let fields = struct_view
            .signature()
            .into_iter()
            .map(|(field_name, field_ty)| {
                let new_ty = self.resolve_expression_type(
                    hir,
                    substitute_type(hir, *field_ty, &subst)?,
                    span,
                )?;
                Ok(Visible::new(field_name.visibility, (field_name.data, new_ty)))
            })
            .collect::<Result<Vec<_>>>()?;

        let mangled_name = mangle_name(hir, name, &args);
        let mangled_symbol = hir.intern_name(&mangled_name);
        let specialized_ty = hir.create_struct_type(mangled_symbol, fields, Vec::new());

        let specialized_local = {
            let file = hir.get_file_mut(template_file);
            file.declarations.declarations.objects.insert(HirObjectDeclaration {
                name: mangled_symbol,
                generics: Vec::new(),
                ty: specialized_ty,
                visibility,
                external,
                attributes: Vec::new(),
            })
        };
        let specialized =
            AnyDeclarationId::new(template_file, AnyLocalDeclarationId::Object(specialized_local));

        self.cache.insert(key.clone(), specialized);
        self.in_progress.remove(&key);
        self.dead_code.insert(template_any);

        Ok(specialized_ty)
    }

    ///Finds the `HirObjectDeclaration` with the given `name` in any file.
    fn find_object_declaration_by_name(
        &self,
        hir: &SlynxHir,
        name: slynx_hir::SymbolPointer,
    ) -> Option<(module_loader::FileId, PoolId<HirObjectDeclaration>)> {
        for file in hir.files.iter() {
            for (id, declaration) in file.declarations.declarations.objects.iter().with_ids() {
                if declaration.name == name {
                    return Some((file.file, id));
                }
            }
        }
        None
    }

    ///Neutralizes every generic object template and marks it as dead.
    pub(crate) fn neutralize_generic_objects(
        &mut self,
        hir: &SlynxHir,
        files: &[module_loader::FileId],
        void_ty: DedupPoolId<HirType>,
    ) {
        for file_id in files {
            let generic_ids: Vec<PoolId<HirObjectDeclaration>> = {
                let file = hir.get_file(*file_id);
                file.declarations
                    .declarations
                    .objects
                    .iter()
                    .with_ids()
                    .filter(|(_, declaration)| !declaration.generics.is_empty())
                    .map(|(id, _)| id)
                    .collect()
            };

            for local_id in generic_ids {
                let mut file = hir.get_file_mut(*file_id);
                file.declarations
                    .declarations
                    .objects
                    .get_mut(local_id)
                    .ty = void_ty;
                self.dead_code.insert(AnyDeclarationId::new(
                    *file_id,
                    AnyLocalDeclarationId::Object(local_id),
                ));
            }
        }
    }
}