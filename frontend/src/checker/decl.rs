use color_eyre::eyre::Result;

use super::TypeChecker;

use crate::hir::{
    TypeId,
    definitions::{ComponentMemberDeclaration, HirDeclaration, HirDeclarationKind},
    types::HirType,
};
impl TypeChecker {
    pub(super) fn check_decl(&mut self, decl: &mut HirDeclaration) -> Result<()> {
        match decl.kind {
            HirDeclarationKind::Function {
                ref mut statements, ..
            } => {
                self.resolve_statments(statements, &decl.ty)?;
            }
            HirDeclarationKind::Object => {}

            HirDeclarationKind::ComponentDeclaration { ref mut props } => {
                let HirType::Component { props: mut typrops } =
                    self.types_module.get_type(&decl.ty).clone()
                else {
                    unreachable!("Component declaration should have type component");
                };
                for prop in props {
                    match prop {
                        ComponentMemberDeclaration::Property {
                            index, value, span, ..
                        } => {
                            if let Some(value) = value {
                                let index = *index;
                                let ty = self.get_type_of_expr(value)?;
                                typrops[index].2 = self.unify(&typrops[index].2, &ty, span)?;
                            }
                        }
                        ComponentMemberDeclaration::Child { .. }
                        | ComponentMemberDeclaration::Specialized(_) => {}
                    }
                }
                *self.types_module.get_type_mut(&decl.ty) = HirType::Component { props: typrops };
            }
        }
        Ok(())
    }
    pub(super) fn resolve_component_members(
        &mut self,
        values: &mut Vec<ComponentMemberDeclaration>,
        target: TypeId,
    ) -> Result<TypeId> {
        for value in values {
            match value {
                ComponentMemberDeclaration::Specialized(spec) => {
                    self.resolve_specialized(spec)?;
                }
                ComponentMemberDeclaration::Property {
                    index, value, span, ..
                } => {
                    if let Some(value) = value {
                        let ty = self.get_type_of_expr(value)?;
                        let HirType::Component { props } =
                            self.types_module.get_type(&target).clone()
                        else {
                            unreachable!(
                                "The type received when resolving component values should be a component one"
                            );
                        };
                        let ty = self.unify(&props[*index].2, &ty, span)?;
                        let HirType::Component { props } = self.types_module.get_type_mut(&target)
                        else {
                            unreachable!(
                                "The type received when resolving component values should be a component one"
                            );
                        };
                        props[*index].2 = ty;
                    }
                }
                ComponentMemberDeclaration::Child { name, values, .. } => {
                    self.resolve_component_members(values, *name)?;
                }
            }
        }
        Ok(target)
    }
}
