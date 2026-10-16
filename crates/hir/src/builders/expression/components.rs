use common::{Span, Spanned, pool::PoolId};
use module_loader::{ASTType, ASTTypeKind};
use slynx_parser::{ComponentExpression, ComponentMemberValue, TypeContext};

use crate::{
    HIRError, HirComponentExpression, PropertyExpression, Result,
    builders::HirQueueBuilder,
    context::HirSymbol,
    term::{TermId, TermNode},
};

use super::{ExpressionBuilder, ExpressionDescriptor};

pub struct ComponentExpressionDescriptor<'a> {
    pub component: &'a ComponentExpression,
    pub span: Span,
    pub context: &'a TypeContext<'a>,
}

impl ExpressionBuilder {
    pub(crate) fn build_component_expression(
        &mut self,
        queue: &HirQueueBuilder,
        ComponentExpressionDescriptor {
            component,
            span,
            context,
        }: ComponentExpressionDescriptor,
    ) -> Result<Spanned<PoolId<HirComponentExpression>>> {
        let name = queue.get_plain_type(component.name).identifier;
        let lowered = if let Some(self_type) = self.self_type {
            queue.lowerer.lower_type_with_self(
                queue,
                self.file(),
                component.name,
                context,
                self_type,
            )?
        } else {
            queue
                .lowerer
                .lower_type(queue, self.file(), component.name, context)?
        };
        let ty = lowered.term;
        let ty_view = queue.hir.view(lowered.term);
        let Some(comp_view) = ty_view.nominal().is_component() else {
            return Err(HIRError::not_a_component(self.file(), lowered.term, span));
        };
        let concrete_name = queue.hir.types.get_component_name(comp_view.data);
        let concrete_owner = queue.hir.types.get_component_owner(comp_view.data);
        if queue
            .hir
            .find_component_by_symbol(HirSymbol {
                owner: concrete_owner,
                term: concrete_name,
            })
            .is_none()
        {
            if let Some(ASTType {
                owner,
                content: ASTTypeKind::Component(comp),
            }) = queue
                .lowerer
                .lookup
                .find_type(concrete_owner, concrete_name)
            {
                let comp = queue.modules.get_entry(owner).component().get(comp);
                queue.enqueue_component(comp, owner)?;
            } else {
                return Err(HIRError::component_not_found(self.file(), name, span));
            }
        }

        let generics: &[TermId] = match ty_view.raw().node() {
            TermNode::Apply { args, .. } => args.as_slice(),
            _ => &[],
        };

        let mut properties = Vec::new();
        let mut children = Vec::new();
        for value in &component.values {
            match value {
                ComponentMemberValue::Assign { prop_name, rhs }
                    if let Some(position) = comp_view
                        .props()
                        .iter()
                        .position(|prop| prop.name == *prop_name) =>
                {
                    let expected = crate::generics::substitute_terms(
                        queue.hir,
                        generics,
                        comp_view.props()[position].ty,
                    );
                    let expr = self.build_expression(
                        queue,
                        ExpressionDescriptor {
                            target: *rhs,
                            expected: Some(expected),
                            context,
                        },
                    )?;
                    properties.push(PropertyExpression::new(position, expr));
                }
                ComponentMemberValue::Assign { prop_name, .. } => {
                    // couldnt find any member with name `prop_name`
                    return Err(HIRError::property_unrecognized(
                        self.file(),
                        ty,
                        vec![*prop_name],
                        span,
                    ));
                }
                ComponentMemberValue::Child(child) => {
                    let child_expr = self.build_component_expression(
                        queue,
                        ComponentExpressionDescriptor {
                            component: child,
                            span,
                            context,
                        },
                    )?;
                    children.push(child_expr);
                }
                ComponentMemberValue::NamedChild { name, child } => {
                    if let Some(position) = comp_view
                        .children()
                        .iter()
                        .position(|child| matches!(child.name, Some(cname) if cname == *name))
                    {
                        let child_expr = self.build_component_expression(
                            queue,
                            ComponentExpressionDescriptor {
                                component: child,
                                span,
                                context,
                            },
                        )?;
                        children.push(child_expr);
                    }
                }
            }
        }

        let id = queue
            .hir
            .store
            .insert_component_expression(HirComponentExpression {
                name: lowered.term,
                properties,
                children,
            });
        Ok(span.make_spanned(id))
    }
}
