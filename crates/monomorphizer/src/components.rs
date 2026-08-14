//! Component monomorphization.
//!
//! A generic component template (`component List<T> { prop items: vector<T> }`)
//! is specialized by [`resolve_component_target`](Monomorphizer::resolve_component_target):
//! for one concrete type-argument list it creates (or retrieves from the cache)
//! a new component type with substituted property types, plus a mangled,
//! non-generic `HirComponentDeclaration` whose property members (default values
//! and the child tree) are rebuilt with the substitution applied.

use common::{
    Span,
    pool::{DedupPoolId, PoolId},
};
use slynx_hir::{
    ComponentMemberDeclaration, ComponentType, HIRError, HirComponentDeclaration, HirType, Result,
    SlynxHir, SymbolPointer,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};

use crate::{
    Monomorphizer,
    types::{MonomorphizationKey, Substitution, mangle_name, substitute_type},
};

impl Monomorphizer {
    ///Given the `HirType::Reference` type of a generic component usage such as
    ///`List<int>`, returns the specialized component type, generating a mangled
    ///`HirComponentDeclaration` on first use and deduplicating identical
    ///instantiations afterwards.
    pub(crate) fn resolve_component_target(
        &mut self,
        hir: &SlynxHir,
        ty: DedupPoolId<HirType>,
        span: Span,
    ) -> Result<DedupPoolId<HirType>> {
        let ty_view = hir.view(ty);
        let HirType::Reference { rf, generics } = ty_view.raw() else {
            unreachable!("resolve_component_target requires a Reference type")
        };
        let ty_view = hir.view(*rf);
        let deref = ty_view.dereference();
        let comp_id = match deref.raw() {
            HirType::Component(id) => *id,
            _ => {
                return Err(HIRError::generic_arity_mismatch(
                    hir.intern_name("<non-component>"),
                    0,
                    generics.iter().filter(|slot| !slot.is_null()).count(),
                    span,
                ));
            }
        };
        let name = hir.intern_name(hir.view(comp_id).name());

        let Some((template_file, template_local)) =
            self.find_component_declaration_by_name(hir, name)
        else {
            unreachable!("Every generic component type must have a HirComponentDeclaration")
        };
        let template_any =
            AnyDeclarationId::new(template_file, AnyLocalDeclarationId::Component(template_local));

        let (template_generics, visibility, template_members) = {
            let file = hir.get_file(template_file);
            let declaration = &file.declarations.declarations.components[template_local];
            (
                declaration.generics.clone(),
                declaration.visibility,
                declaration.props.clone(),
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
            let AnyLocalDeclarationId::Component(local_id) = cached.local_id else {
                unreachable!("A monomorphized component target must be a component")
            };
            return Ok(hir.get_file(cached.file_id).declarations.declarations.components[local_id]
                .ty);
        }
        if self.in_progress.contains(&key) {
            return Err(HIRError::cyclic_monomorphization(name, args, span));
        }
        self.in_progress.insert(key.clone());

        let subst = Substitution::new(&template_generics, &args);
        let specialized_ty = self.rebuild_component_type(hir, comp_id, &subst, span)?;

        let mangled_name = mangle_name(hir, name, &args);
        let mangled_symbol = hir.intern_name(&mangled_name);
        let new_members = self.build_component_members(hir, &template_members, &subst)?;

        let specialized_local = {
            let file = hir.get_file_mut(template_file);
            file.declarations
                .declarations
                .components
                .insert(HirComponentDeclaration {
                    name: mangled_symbol,
                    generics: Vec::new(),
                    props: new_members,
                    ty: specialized_ty,
                    visibility,
                    attributes: Vec::new(),
                })
        };
        let specialized = AnyDeclarationId::new(
            template_file,
            AnyLocalDeclarationId::Component(specialized_local),
        );

        self.cache.insert(key.clone(), specialized);
        self.in_progress.remove(&key);
        self.dead_code.insert(template_any);

        Ok(specialized_ty)
    }

    ///Rebuilds a component type with `subst` applied to its property types and
    ///its children, resolving any generic object/component references left
    ///over. Returns a fresh component type id.
    pub(crate) fn rebuild_component_type(
        &mut self,
        hir: &SlynxHir,
        comp_ty: DedupPoolId<ComponentType>,
        subst: &Substitution,
        span: Span,
    ) -> Result<DedupPoolId<HirType>> {
        let view = hir.view(comp_ty);
        let name = hir.intern_name(view.name());

        let properties = view
            .props()
            .iter()
            .zip(view.prop_names())
            .map(|(prop_ty, prop_name)| {
                let new_ty = self.resolve_expression_type(
                    hir,
                    substitute_type(hir, *prop_ty, subst)?,
                    span,
                )?;
                Ok((*prop_name, new_ty))
            })
            .collect::<Result<Vec<_>>>()?;

        let children = view
            .children()
            .iter()
            .map(|child| {
                let child_ty = self.rebuild_component_type(hir, *child, subst, span)?;
                let child_view = hir.view(child_ty);
                let HirType::Component(child_id) = child_view.raw() else {
                    unreachable!("rebuild_component_type must yield a component type")
                };
                Ok(*child_id)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(hir.create_component_type(name, properties, children))
    }

    ///Rebuilds the member list of a component declaration, substituting the
    ///generic parameters inside property default values and the child tree.
    fn build_component_members(
        &mut self,
        hir: &SlynxHir,
        members: &[ComponentMemberDeclaration],
        subst: &Substitution,
    ) -> Result<Vec<ComponentMemberDeclaration>> {
        members
            .iter()
            .map(|member| match member {
                ComponentMemberDeclaration::Property {
                    name,
                    modifier,
                    index,
                    value,
                    span,
                } => {
                    let value = value
                        .map(|expr| self.build_expression(hir, expr, subst))
                        .transpose()?;
                    Ok(ComponentMemberDeclaration::Property {
                        name: *name,
                        modifier: *modifier,
                        index: *index,
                        value,
                        span: *span,
                    })
                }
                ComponentMemberDeclaration::Child(child) => {
                    let child = self.build_component_expression(hir, *child, subst)?;
                    Ok(ComponentMemberDeclaration::Child(child))
                }
            })
            .collect()
    }

    ///Rewrites the members of a non-generic component, resolving any generic
    ///object/component usage inside default values and the child tree.
    pub(crate) fn rewrite_non_generic_component(
        &mut self,
        hir: &SlynxHir,
        file_id: module_loader::FileId,
        local_id: PoolId<HirComponentDeclaration>,
    ) -> Result<()> {
        let template_members = {
            let file = hir.get_file(file_id);
            file.declarations.declarations.components[local_id].props.clone()
        };
        let new_members = self.build_component_members(hir, &template_members, &Substitution::empty())?;
        let mut file = hir.get_file_mut(file_id);
        file.declarations
            .declarations
            .components
            .get_mut(local_id)
            .props = new_members;
        Ok(())
    }

    ///Finds the `HirComponentDeclaration` with the given `name` in any file.
    fn find_component_declaration_by_name(
        &self,
        hir: &SlynxHir,
        name: SymbolPointer,
    ) -> Option<(module_loader::FileId, PoolId<HirComponentDeclaration>)> {
        for file in hir.files.iter() {
            for (id, declaration) in file.declarations.declarations.components.iter().with_ids() {
                if declaration.name == name {
                    return Some((file.file, id));
                }
            }
        }
        None
    }

    ///Neutralizes every generic component template and marks it as dead.
    pub(crate) fn neutralize_generic_components(
        &mut self,
        hir: &SlynxHir,
        files: &[module_loader::FileId],
        void_ty: DedupPoolId<HirType>,
    ) {
        for file_id in files {
            let generic_ids: Vec<PoolId<HirComponentDeclaration>> = {
                let file = hir.get_file(*file_id);
                file.declarations
                    .declarations
                    .components
                    .iter()
                    .with_ids()
                    .filter(|(_, declaration)| !declaration.generics.is_empty())
                    .map(|(id, _)| id)
                    .collect()
            };

            for local_id in generic_ids {
                let mut file = hir.get_file_mut(*file_id);
                let declaration = file.declarations.declarations.components.get_mut(local_id);
                declaration.props = Vec::new();
                declaration.ty = void_ty;
                self.dead_code.insert(AnyDeclarationId::new(
                    *file_id,
                    AnyLocalDeclarationId::Component(local_id),
                ));
            }
        }
    }
}