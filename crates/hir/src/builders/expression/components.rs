use common::{Span, Spanned, pool::PoolId};
use module_loader::{ASTType, ASTTypeKind};
use slynx_parser::{ComponentExpression, ComponentMemberValue, TypeContext};

use crate::{
    HIRError, HirComponentExpression, PropertyExpression, Result, builders::HirQueueBuilder,
    context::HirSymbol,
};

use super::ExpressionBuilder;

impl ExpressionBuilder {
    pub(crate) fn build_component_expression(
        &mut self,
        queue: &HirQueueBuilder,
        component: &ComponentExpression,
        span: Span,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirComponentExpression>>> {
        let name = queue.get_plain_type(component.name).identifier;
        let node = queue.get_node(self.file());
        let (owner, ty) = node.find_type(component.name, context)?;
        if queue
            .hir
            .find_component_by_symbol(HirSymbol::new(owner, name))
            .is_none()
        {
            let ty = queue.modules.find_type_inside_module(self.file(), name);
            if let Some(ASTType {
                content: ASTTypeKind::Component(comp),
                ..
            }) = ty
            {
                queue.enqueue_component(comp, self.file())?;
            } else {
                return Err(HIRError::component_not_found(name, span));
            }
        }
        let ty_view = queue.hir.view(ty);
        let deref = ty_view.dereference();
        let comp_view = deref
            .is_component()
            .expect("find_type returned non-component type for component name");

        let mut properties = Vec::new();
        let mut children = Vec::new();
        for value in &component.values {
            match value {
                ComponentMemberValue::Assign { prop_name, rhs } => {
                    let pos = comp_view
                        .prop_names()
                        .iter()
                        .position(|n| n == prop_name)
                        .ok_or_else(|| {
                            HIRError::property_unrecognized(ty, vec![*prop_name], span)
                        })?;
                    let expr =
                        self.build_expression(queue, *rhs, Some(comp_view.props()[pos]), context)?;
                    properties.push(PropertyExpression::new(pos, expr));
                }
                ComponentMemberValue::Child(child) => {
                    let child_expr =
                        self.build_component_expression(queue, child, span, context)?;
                    children.push(child_expr);
                }
            }
        }

        let component_expr = HirComponentExpression {
            name: ty,
            properties,
            children,
        };
        let id = queue.hir.insert_component_expression(component_expr);
        Ok(span.make_spanned(id))
    }
}
