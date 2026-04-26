///Module that implements anything related Specialized Component on the HIR
use common::{ComponentExpression, ComponentMemberValue, Span};

use crate::hir::{
    Result, SlynxHir,
    error::{HIRError, HIRErrorKind},
    model::{ComponentMemberDeclaration, SpecializedComponent},
};

impl SlynxHir {
    pub fn resolve_specialize_text(
        &mut self,
        values: Vec<ComponentMemberValue>,
        span: &Span,
    ) -> Result<SpecializedComponent> {
        let mut text = None;
        for value in values {
            match value {
                ComponentMemberValue::Assign {
                    prop_name,
                    rhs,
                    span,
                } => match prop_name.as_str() {
                    "text" => text = Some(self.resolve_expr(rhs, None)?),
                    _ => {
                        let intern = self.symbols_module.intern(&prop_name);
                        return Err(HIRError::type_unrecognized(intern, span).into());
                    }
                },
                ComponentMemberValue::Child(e) => {
                    return Err(HIRError {
                        kind: HIRErrorKind::InvalidChild { child: Box::new(e) },
                        span: span.clone(),
                    }
                    .into());
                }
            }
        }
        if let Some(text) = text {
            Ok(SpecializedComponent::Text {
                text: Box::new(text),
            })
        } else {
            let properties = vec![self.symbols_module.intern("text")];
            Err(HIRError::missing_properties(properties, *span).into())
        }
    }
    ///Resolves the provided `children` knowning it is a specialized div component
    pub fn resolve_specialized_div(
        &mut self,
        children: Vec<ComponentMemberValue>,
        _: &Span,
    ) -> Result<SpecializedComponent> {
        let mut out = Vec::with_capacity(children.len());

        for child in children {
            match child {
                ComponentMemberValue::Assign {
                    prop_name, span, ..
                } => {
                    let prop = self.symbols_module.intern(&prop_name);
                    return Err(HIRError::property_unrecognized(vec![prop], span).into());
                }
                ComponentMemberValue::Child(c) => {
                    let component = self.resolve_component(c)?;
                    out.push(component);
                }
            }
        }

        Ok(SpecializedComponent::Div { children: out })
    }
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
    ///Resolves the provided `component` expression. If it's a specialized one, resolves as a SpecializedComponent, otherwise as a normal 'Component'
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
                Ok(ComponentMemberDeclaration::Child {
                    name: id,
                    values,
                    span: component.span,
                })
            }
            (_, _) => unreachable!(
                "Try resolve specialized is bugged. This should literally never happen"
            ),
        }
    }
}
