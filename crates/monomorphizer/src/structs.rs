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

use common::{Span, pool::DedupPoolId};
use slynx_hir::{
    HIRError, HirObjectDeclaration, HirType, Result, SlynxHir, Visible,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
    term::TermNode,
};

use crate::{Monomorphizer, types::substitute_type};

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
        let TermNode::Apply { target, args } = ty_view.raw().node() else {
            unreachable!("resolve_object_target requires a Reference type")
        };

        let rf_view = hir.view(*target);
        let deref = rf_view.dereference();
        let struct_view = deref.is_struct().ok_or_else(|| {
            HIRError::generic_arity_mismatch(
                hir.intern_name(&deref.name()),
                0,
                args.iter().filter(|slot| !slot.is_null()).count(),
                span,
            )
        })?;
        let name = struct_view.name();

        let Some((template_file, template_local)) =
            self.find_declaration_by_name(hir, name, |pool| &pool.objects, |d| d.name)
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

        let args: Vec<DedupPoolId<HirType>> = args
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
                let AnyLocalDeclarationId::Object(local_id) = cached.local_id else {
                    unreachable!("A monomorphized object target must be an object")
                };
                hir.get_file(cached.file_id)
                    .declarations
                    .declarations
                    .objects[local_id]
                    .ty
            },
            |monomorphizer, hir, subst, mangled_symbol, _| {
                let fields = struct_view
                    .signature()
                    .into_iter()
                    .map(|(field_name, field_ty)| {
                        let new_ty = monomorphizer.resolve_expression_type(
                            hir,
                            substitute_type(hir, *field_ty, subst)?,
                            span,
                        )?;
                        Ok(Visible::new(
                            field_name.visibility,
                            (field_name.data, new_ty),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;

                let specialized_ty =
                    hir.types
                        .create_struct_type(mangled_symbol, fields, Vec::new());

                let specialized_local = {
                    let file = hir.get_file_mut(template_file);
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

                Ok((
                    AnyDeclarationId::new(
                        template_file,
                        AnyLocalDeclarationId::Object(specialized_local),
                    ),
                    specialized_ty,
                ))
            },
        )
    }
}
