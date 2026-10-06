//! Component monomorphization.
//!
//! A generic component template (`component List<T> { prop items: vector<T> }`)
//! is specialized by [`resolve_component_target`](Monomorphizer::resolve_component_target):
//! for one concrete type-argument list it creates (or retrieves from the cache)
//! a new component type with substituted property types, plus a mangled,
//! non-generic `HirComponentDeclaration` whose property members (default values
//! and the child tree) are rebuilt with the substitution applied.
//!
//! This module also owns the neutralization of the component templates that
//! survive the pass
//! ([`neutralize_generic_components`](Monomorphizer::neutralize_generic_components)).

use common::{
    Span,
    pool::{DedupPoolId, PoolId},
};
use module_loader::FileId;
use slynx_hir::{
    ComponentMemberDeclaration, ComponentType, DeclarationId, DescriptorId, HIRError,
    HirComponentDeclaration, Result, SlynxHir, SymbolPointer,
    term::{TermId, TermNode},
};

use crate::{
    Monomorphizer,
    specialization::SpecializationDescriptor,
    types::{MonomorphizationKey, Substitution, substitute_type},
};

impl Monomorphizer {
    ///Given the `HirType::Reference` type of a generic component usage such as
    ///`List<int>`, returns the specialized component type, generating a mangled
    ///`HirComponentDeclaration` on first use and deduplicating identical
    ///instantiations afterwards.
    ///
    ///The request is handed to the shared
    ///[`specialize`](Monomorphizer::specialize) skeleton as a
    ///[`SpecializationDescriptor`]; the skeleton reads the template's name and
    ///generic arity itself, and the resulting typed id is narrowed to the
    ///specialization's component type for the caller.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR being monomorphized; the declaration is inserted into
    ///  the template's file so codegen hoists it next to its template.
    ///* `ty` — the concrete type application (`List<int>`) to specialize.
    ///* `span` — the use-site span, reported on arity and cycle errors.
    ///
    ///# Returns
    ///
    ///The component type of the concrete copy — freshly generated or served
    ///from the cache.
    pub(crate) fn resolve_component_target(
        &mut self,
        hir: &SlynxHir,
        ty: TermId,
        span: Span,
    ) -> Result<TermId> {
        let ty_view = hir.view(ty);
        let TermNode::Apply { target, args } = ty_view.raw().node() else {
            unreachable!("resolve_component_target requires a Reference type")
        };
        let ty_view = hir.view(*target);
        let deref = ty_view.dereference();
        let comp_id = match deref.raw().node() {
            TermNode::Data(DescriptorId::Component(c)) => *c,
            _ => {
                return Err(HIRError::generic_arity_mismatch(
                    hir.intern_name("<non-component>"),
                    0,
                    args.iter().filter(|slot| !slot.is_null()).count(),
                    span,
                ));
            }
        };
        let name = hir.intern_name(hir.view(comp_id).name());

        let Some(template) = self.find_declaration_by_name::<HirComponentDeclaration>(hir, name)
        else {
            unreachable!("Every generic component type must have a HirComponentDeclaration")
        };
        let owner = template.owner;

        let (visibility, template_members) = {
            let file = hir.get_file(owner);
            let declaration = &file.declarations.declarations.components[template.term];
            (declaration.visibility, declaration.props.clone())
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
                    let specialized_ty =
                        monomorphizer.rebuild_component_type(hir, comp_id, subst, span)?;

                    let new_members =
                        monomorphizer.build_component_members(hir, &template_members, subst)?;

                    let specialized_local = {
                        let file = hir.get_file_mut(owner);
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

                    Ok(DeclarationId::new(owner, specialized_local))
                },
            },
        )?;

        let file = hir.get_file(specialized.owner);
        Ok(file.declarations.declarations.components[specialized.term].ty)
    }

    ///Neutralizes every generic *component* template: empties its member list
    ///and retypes it to `void_ty`, then records it as dead code.
    ///
    ///After this runs no component declaration in the HIR carries a
    ///`GenericParam`-typed signature, so codegen never sees one. Templates
    ///that were instantiated during the pass are neutralized too — their
    ///concrete specializations exist alongside them and are the declarations
    ///codegen emits.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose component templates are neutralized.
    ///* `files` — the files to scan for generic templates.
    ///* `void_ty` — the concrete, non-generic type every neutralized template
    ///  is retyped to.
    pub(crate) fn neutralize_generic_components(
        &mut self,
        hir: &SlynxHir,
        files: &[FileId],
        void_ty: TermId,
    ) {
        for template in self.generic_templates::<HirComponentDeclaration>(hir, files) {
            let mut file = hir.get_file_mut(template.owner);
            let declaration = file
                .declarations
                .declarations
                .components
                .get_mut(template.term);
            declaration.props = Vec::new();
            declaration.ty = void_ty;
            self.mark_dead(template);
        }
    }

    ///Rebuilds a component type with `subst` applied to its property types and
    ///its children, resolving any generic object/component references left
    ///over. Returns a fresh component type id.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose type pool receives the rebuilt component type.
    ///* `comp_ty` — the component type to rebuild.
    ///* `subst` — the substitution of generic parameters for this
    ///  instantiation.
    ///* `span` — the use-site span, forwarded when nested property types still
    ///  reference a specialization.
    pub(crate) fn rebuild_component_type(
        &mut self,
        hir: &SlynxHir,
        comp_ty: DedupPoolId<ComponentType>,
        subst: &Substitution,
        span: Span,
    ) -> Result<TermId> {
        let view = hir.view(comp_ty);
        let name = hir.intern_name(view.name());

        let properties = view
            .props()
            .iter()
            .map(|prop| {
                let new_ty =
                    self.resolve_expression_type(hir, substitute_type(hir, prop.ty, subst)?, span)?;
                Ok((prop.name, new_ty))
            })
            .collect::<Result<Vec<_>>>()?;

        let children = view
            .children()
            .iter()
            .map(|child| {
                let child_ty = self.rebuild_component_type(hir, *child, subst, span)?;
                let child_view = hir.view(child_ty);
                if let TermNode::Data(descriptor) = child_view.raw().node()
                    && let DescriptorId::Component(child) = descriptor
                {
                    Ok(*child)
                } else {
                    unreachable!("rebuild_component_type must yield a component type")
                }
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(hir.types.create_component_type(name, properties, children))
    }

    ///Rebuilds the member list of a component declaration, substituting the
    ///generic parameters inside property default values and the child tree.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose expression pools receive the rebuilt members.
    ///* `members` — the member list to rebuild.
    ///* `subst` — the substitution of generic parameters for this
    ///  instantiation.
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
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose expression pools receive the rebuilt members.
    ///* `owner` — the file the component lives in.
    ///* `local_id` — the pool id of the non-generic component to rewrite.
    pub(crate) fn rewrite_non_generic_component(
        &mut self,
        hir: &SlynxHir,
        owner: FileId,
        local_id: PoolId<HirComponentDeclaration>,
    ) -> Result<()> {
        let template_members = {
            let file = hir.get_file(owner);
            file.declarations.declarations.components[local_id]
                .props
                .clone()
        };
        let new_members =
            self.build_component_members(hir, &template_members, &Substitution::empty())?;
        let mut file = hir.get_file_mut(owner);
        file.declarations
            .declarations
            .components
            .get_mut(local_id)
            .props = new_members;
        Ok(())
    }
}
