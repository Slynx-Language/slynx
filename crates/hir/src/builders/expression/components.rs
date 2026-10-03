use common::{Span, Spanned, pool::PoolId};
use module_loader::{ASTType, ASTTypeKind};
use slynx_parser::{ComponentExpression, ComponentMemberValue, TypeContext};

use crate::{
    HIRError, HirComponentExpression, PropertyExpression, Result, builders::HirQueueBuilder,
    context::HirSymbol,
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
        let lowered = queue
            .lowerer
            .lower_type(queue, self.file(), component.name, context)?;
        let ty = lowered.term;
        if queue
            .hir
            .find_component_by_symbol(HirSymbol::new(lowered.owner, name))
            .is_none()
        {
            if let Some(ASTType {
                owner,
                content: ASTTypeKind::Component(comp),
            }) = queue.lowerer.lookup.find_type(self.file(), name)
            {
                let comp = queue.modules.get_entry(owner).component().get(comp);
                queue.enqueue_component(comp, owner)?;
            } else {
                return Err(HIRError::component_not_found(name, span));
            }
        }
        let ty_view = queue.hir.view(lowered.term);
        let deref = ty_view.dereference();
        let comp_view = deref
            .is_component()
            .expect("find_type returned non-component type for component name");

        let mut properties = Vec::new();
        let mut children = Vec::new();
        for value in &component.values {
            match value {
                ComponentMemberValue::Assign { prop_name, rhs }
                    if let Some(position) =
                        comp_view.prop_names().iter().position(|n| n == prop_name) =>
                {
                    let expr = self.build_expression(
                        queue,
                        ExpressionDescriptor {
                            target: *rhs,
                            expected: Some(comp_view.props()[position]),
                            context,
                        },
                    )?;
                    properties.push(PropertyExpression::new(position, expr));
                }
                ComponentMemberValue::Assign { prop_name, .. } => {
                    // couldnt find any member with name `prop_name`
                    return Err(HIRError::property_unrecognized(ty, vec![*prop_name], span));
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
