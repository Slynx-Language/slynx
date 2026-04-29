///Module that implements anything related Specialized Component on the HIR
use common::{ComponentExpression, ComponentMemberValue, Span, VisibilityModifier};

use crate::hir::{
    Result, SlynxHir, TypeId,
    error::{HIRError, HIRErrorKind},
    model::{ComponentMemberDeclaration, HirType, SpecializedComponent},
};

impl SlynxHir {
    fn resolve_component_member(
        &mut self,
        member: ComponentMemberValue,
        ty: TypeId,
    ) -> Result<ComponentMemberDeclaration> {
        match member {
            ComponentMemberValue::Assign {
                prop_name,
                rhs,
                span,
            } => {
                let interned_name = self.modules.intern_name(&prop_name);
                let HirType::Component { props } = self.get_type(&ty) else {
                    unreachable!("The type should be a component instead");
                };
                match props.iter().position(|prop| prop.name() == prop_name) {
                    None => Err(HIRError::name_unrecognized(interned_name, span)),
                    Some(index)
                        if matches!(
                            props[index].visibility(),
                            VisibilityModifier::Private | VisibilityModifier::ChildrenPublic
                        ) =>
                    {
                        Err(HIRError::not_visible_property(interned_name, span))
                    }

                    Some(index) => {
                        let expr = self.resolve_expr(rhs, Some(*props[index].prop_type()))?;
                        Ok(ComponentMemberDeclaration::new_property(
                            index,
                            Some(expr),
                            span,
                        ))
                    }
                }
            }
            ComponentMemberValue::Child(child) => {
                //By now this won't track whether it can or cannot have children, since a method better than 'children' might be implemented in the future.
                let (id, _) =
                    self.retrieve_information_of_type(&child.name.identifier, &child.name.span)?;
                let values = self.resolve_component_members(child.values, id)?;
                Ok(ComponentMemberDeclaration::new_child(
                    id, values, child.span,
                ))
            }
        }
    }

    /// Resolves the provided values on a component. The `ty` is the type of the component we are resolving it
    pub fn resolve_component_members(
        &mut self,
        members: Vec<ComponentMemberValue>,
        ty: TypeId,
    ) -> Result<Vec<ComponentMemberDeclaration>> {
        let out = members
            .into_iter()
            .map(|member| self.resolve_component_member(member, ty))
            .collect::<Result<Vec<_>>>()?;
        Ok(out)
    }

    pub fn resolve_specialize_text(
        &mut self,
        values: Vec<ComponentMemberValue>,
        span: &Span,
    ) -> Result<SpecializedComponent> {
        let mut text = None;
        for value in values {
            match value {
                ComponentMemberValue::Assign { prop_name, rhs, .. } if prop_name == "text" => {
                    text = Some(self.resolve_expr(rhs, None)?)
                }
                ComponentMemberValue::Assign {
                    prop_name, span, ..
                } => {
                    let intern = self.modules.intern_name(&prop_name);
                    return Err(HIRError::type_unrecognized(intern, span));
                }
                ComponentMemberValue::Child(e) => {
                    return Err(HIRError {
                        kind: HIRErrorKind::InvalidChild { child: Box::new(e) },
                        span: *span,
                    });
                }
            }
        }
        match text {
            Some(text) => Ok(SpecializedComponent::new_text(text)),
            None => {
                let properties = vec![self.modules.intern_name("text")];
                Err(HIRError::missing_properties(properties, *span))
            }
        }
    }

    ///Resolves the provided `children` knowning it is a specialized div component
    pub fn resolve_specialized_div(
        &mut self,
        children: Vec<ComponentMemberValue>,
        _: &Span,
    ) -> Result<SpecializedComponent> {
        let children = children
            .into_iter()
            .map(|c| match c {
                ComponentMemberValue::Assign {
                    prop_name, span, ..
                } => {
                    let prop = self.modules.intern_name(&prop_name);
                    Err(HIRError::property_unrecognized(vec![prop], span))
                }
                ComponentMemberValue::Child(c) => self.resolve_component(c),
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(SpecializedComponent::new_div(children))
    }
    ///Tries to resolve the given `child` as, either a specialized component, or a normal user defined component
    pub fn try_resolve_specialized(
        &mut self,
        child: ComponentExpression,
    ) -> (
        Option<Result<SpecializedComponent>>,
        Option<ComponentExpression>,
    ) {
        match (child.name.identifier.as_str(), &child.name.generic) {
            ("Text", None) => (
                Some(self.resolve_specialize_text(child.values, &child.span)),
                None,
            ),
            ("Div", None) => (
                Some(self.resolve_specialized_div(child.values, &child.span)),
                None,
            ),
            _ => (None, Some(child)),
        }
    }

    ///Resolves the provided `component` expression. If it's a specialized one, resolves as a `SpecializedComponent`, otherwise as a normal 'Component'
    pub fn resolve_component(
        &mut self,
        component: ComponentExpression,
    ) -> Result<ComponentMemberDeclaration> {
        match self.try_resolve_specialized(component) {
            (Some(spec), None) => spec.map(ComponentMemberDeclaration::Specialized),
            (None, Some(component)) => {
                let (id, _) =
                    self.retrieve_information_of_type(&component.name.identifier, &component.span)?;
                let values = self.resolve_component_members(component.values, id)?;
                Ok(ComponentMemberDeclaration::new_child(
                    id,
                    values,
                    component.span,
                ))
            }
            (_, _) => unreachable!(
                "Try resolve specialized is bugged. This should literally never happen"
            ),
        }
    }
}
