//! Function monomorphization.
//!
//! A generic function template (`func identity<T>(x: T): T`) is specialized by
//! [`resolve_function_target`](Monomorphizer::resolve_function_target): for one
//! concrete type-argument list it creates (or retrieves from the cache) a
//! concrete, mangled `HirFunctionDeclaration` whose signature and body have
//! every generic parameter substituted.

use common::{Span, pool::DedupPoolId};
use slynx_hir::{
    DeclarationId, HirFunctionDeclaration, HirType, Result, SlynxHir,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};

use crate::{Monomorphizer, types::substitute_type};

impl Monomorphizer {
    ///Generates (or retrieves from the cache) the specialization of the generic
    ///`template` function with the given concrete type `args`.
    pub(crate) fn resolve_function_target(
        &mut self,
        hir: &SlynxHir,
        template: DeclarationId<HirFunctionDeclaration>,
        args: Vec<DedupPoolId<HirType>>,
        span: Span,
    ) -> Result<AnyDeclarationId> {
        let template_any = AnyDeclarationId::new(
            template.file_id,
            AnyLocalDeclarationId::Function(template.local_id),
        );

        let (name, generics, fargs, fty, statements, visibility, external, attributes) = {
            let file = hir.get_file(template.file_id);
            let declaration = &file.declarations.declarations.functions[template.local_id];
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
                    let file = hir.get_file_mut(template.file_id);
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
                    template.file_id,
                    AnyLocalDeclarationId::Function(specialized_local),
                );

                // Cache *before* generating the body so recursive instantiations
                // of the same (template, args) resolve to this very declaration.
                monomorphizer.cache.insert(key.clone(), specialized);

                let new_statements = monomorphizer.build_statements(hir, &statements, subst)?;
                {
                    let mut file = hir.get_file_mut(template.file_id);
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
    ) -> Result<DedupPoolId<HirType>> {
        let AnyLocalDeclarationId::Function(local_id) = id.local_id else {
            unreachable!("A monomorphized call target must be a function")
        };
        let file = hir.get_file(id.file_id);
        let declaration = &file.declarations.declarations.functions[local_id];
        let view = hir.view(declaration.ty);
        let function = view
            .is_function()
            .expect("Function declaration should have a function type");
        Ok(function.return_type())
    }
}
