/*!
Type checking for top-level declarations.

This module contains the logic to type-check HIR declarations:
- Functions: type-check all statements in the function body.
- Objects/Aliases: no-op at this stage (handled elsewhere).
- Components: check property initializers and unify them with the declared
  component prop types. We read the component type, perform unifications
  locally and then write the updated component type back into the
  `TypesModule` to avoid borrow conflicts.

The implementation intentionally clones the component type's `props` before
resolving expressions so we don't hold a long-lived borrow on the
`TypesModule` while performing potentially mutating operations that also
access the `TypesModule`.
*/

use color_eyre::eyre::Result;

use super::TypeChecker;

use crate::hir::{
    TypeId,
    model::{ComponentMemberDeclaration, HirDeclaration, HirDeclarationKind, HirType},
};

impl TypeChecker {
    /// Type-check a single top-level declaration.
    ///
    /// For functions this resolves statements; for components it resolves
    /// property initializers and updates the component type accordingly.
    pub(super) fn check_decl(&mut self, decl: &mut HirDeclaration) -> Result<()> {
        match &mut decl.kind {
            HirDeclarationKind::Function { statements, .. } => {
                self.resolve_statements(statements, &decl.ty)?;
            }

            // Objects and aliases have no bodies to type-check at this pass.
            HirDeclarationKind::Object => {}
            HirDeclarationKind::Alias => {}

            HirDeclarationKind::ComponentDeclaration { props, .. } => {
                let ty = self.types_module.get_type(&decl.ty).clone();
                let HirType::Component {
                    props: mut declared_props,
                } = ty
                else {
                    unreachable!("Component declaration should have component HirType");
                };

                for member in props.iter_mut() {
                    match member {
                        ComponentMemberDeclaration::Property {
                            index,
                            value: maybe_expr,
                            span,
                            ..
                        } => {
                            if let Some(expr) = maybe_expr {
                                let expr_ty = self.get_type_of_expr(expr)?;

                                *declared_props[*index].prop_type_mut() =
                                    self.unify(declared_props[*index].prop_type(), &expr_ty, span)?;
                            }
                        }

                        ComponentMemberDeclaration::Child { .. } => {}

                        ComponentMemberDeclaration::Specialized(_) => {}
                    }
                }

                *self.types_module.get_type_mut(&decl.ty) = HirType::Component {
                    props: declared_props,
                };
            }
        }

        Ok(())
    }

    /// Resolve and unify a list of component members against a target component type.
    ///
    /// This function is used when checking component instances: it walks the provided
    /// members (properties, children and specializations), resolves values and
    /// unifies them against the `target` component's declared prop types.
    ///
    /// Returns the `target` TypeId on success.
    pub(super) fn resolve_component_members(
        &mut self,
        values: &mut [ComponentMemberDeclaration],
        target: TypeId,
    ) -> Result<TypeId> {
        let target_ty = self.types_module.get_type(&target).clone();
        let HirType::Component {
            props: mut declared_props,
        } = target_ty
        else {
            unreachable!("Expected component type when resolving component members");
        };

        for value in values.iter_mut() {
            match value {
                ComponentMemberDeclaration::Specialized(spec) => {
                    self.resolve_specialized(spec)?;
                }

                ComponentMemberDeclaration::Property {
                    index,
                    value: maybe_expr,
                    span,
                    ..
                } => {
                    if let Some(expr) = maybe_expr {
                        let expr_ty = self.get_type_of_expr(expr)?;
                        let unified =
                            self.unify(declared_props[*index].prop_type(), &expr_ty, span)?;

                        *declared_props[*index].prop_type_mut() = unified;
                    }
                }

                ComponentMemberDeclaration::Child {
                    name,
                    values: child_values,
                    ..
                } => {
                    self.resolve_component_members(child_values, *name)?;
                }
            }
        }

        *self.types_module.get_type_mut(&target) = HirType::Component {
            props: declared_props,
        };

        Ok(target)
    }
}
