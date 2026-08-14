//! Function monomorphization.
//!
//! A generic function template (`func identity<T>(x: T): T`) is specialized by
//! [`resolve_function_target`](Monomorphizer::resolve_function_target): for one
//! concrete type-argument list it creates (or retrieves from the cache) a
//! concrete, mangled `HirFunctionDeclaration` whose signature and body have
//! every generic parameter substituted.

use common::{
    Span,
    pool::{DedupPoolId, PoolId},
};
use slynx_hir::{
    DeclarationId, HirFunctionDeclaration, HirType, Result, SlynxHir,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};

use crate::{
    Monomorphizer,
    types::{MonomorphizationKey, Substitution, mangle_name, substitute_type},
};

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

        if generics.len() != args.len() {
            return Err(slynx_hir::HIRError::generic_arity_mismatch(
                name,
                generics.len(),
                args.len(),
                span,
            ));
        }

        let key: MonomorphizationKey = (template_any, args.clone().into());
        if let Some(cached) = self.cache.get(&key) {
            return Ok(*cached);
        }
        if self.in_progress.contains(&key) {
            return Err(slynx_hir::HIRError::cyclic_monomorphization(name, args, span));
        }
        self.in_progress.insert(key.clone());

        let subst = Substitution::new(&generics, &args);
        let specialized_ty = self.resolve_expression_type(
            hir,
            substitute_type(hir, fty, &subst)?,
            span,
        )?;
        let mangled_name = mangle_name(hir, name, &args);
        let mangled_symbol = hir.intern_name(&mangled_name);

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
                })
        };
        let specialized = AnyDeclarationId::new(
            template.file_id,
            AnyLocalDeclarationId::Function(specialized_local),
        );

        // Cache *before* generating the body so recursive instantiations of the
        // same (template, args) resolve to this very declaration.
        self.cache.insert(key.clone(), specialized);

        let new_statements = self.build_statements(hir, &statements, &subst)?;
        {
            let mut file = hir.get_file_mut(template.file_id);
            file.declarations
                .declarations
                .functions
                .get_mut(specialized_local)
                .statements = new_statements;
        }

        self.in_progress.remove(&key);
        self.dead_code.insert(template_any);

        Ok(specialized)
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

    ///Neutralizes every generic function template so codegen never sees a
    ///`GenericParam`-typed signature, and marks it as dead.
    pub(crate) fn neutralize_generic_functions(
        &mut self,
        hir: &SlynxHir,
        files: &[module_loader::FileId],
        void_ty: DedupPoolId<HirType>,
    ) {
        for file_id in files {
            let generic_ids: Vec<PoolId<HirFunctionDeclaration>> = {
                let file = hir.get_file(*file_id);
                file.declarations
                    .declarations
                    .functions
                    .iter()
                    .with_ids()
                    .filter(|(_, declaration)| !declaration.generics.is_empty())
                    .map(|(id, _)| id)
                    .collect()
            };

            for local_id in generic_ids {
                let mut file = hir.get_file_mut(*file_id);
                let declaration = file.declarations.declarations.functions.get_mut(local_id);
                declaration.statements = Vec::new();
                declaration.ty = void_ty;
                self.dead_code.insert(AnyDeclarationId::new(
                    *file_id,
                    AnyLocalDeclarationId::Function(local_id),
                ));
            }
        }
    }
}