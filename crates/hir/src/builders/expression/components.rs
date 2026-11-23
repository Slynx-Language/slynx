use common::{Span, Spanned, pool::PoolId};
use module_loader::{ASTType, ASTTypeKind};
use slynx_parser::{ComponentExpression, ComponentMemberValue, TypeContext};

use crate::{
    ComponentChildExpression, ComponentMemberDeclaration, DeclarationId, HIRError, HIRErrorKind,
    HirComponentDeclaration, HirComponentExpression, PropertyExpression, Result,
    builders::HirQueueBuilder,
    context::HirSymbol,
    generics::GenericTypeArguments,
    term::{TermId, TermNode},
};

use super::{ExpressionBuilder, ExpressionDescriptor};

pub struct ComponentExpressionDescriptor<'a> {
    pub component: &'a ComponentExpression,
    pub span: Span,
    ///The type expected at this position, if any. A bare component literal has
    ///no explicit type arguments of its own, so this is where the concrete
    ///arguments of an expected `Box<int>` come from before inference fills in
    ///whatever the properties themselves can determine.
    pub expected: Option<TermId>,
    pub context: &'a TypeContext<'a>,
}

impl ExpressionBuilder {
    pub(crate) fn get_or_enqueue_component(
        &mut self,
        queue: &HirQueueBuilder,
        symbol: HirSymbol,
        span: Span,
    ) -> crate::Result<DeclarationId<HirComponentDeclaration>> {
        match queue.hir.find_component_by_symbol(symbol) {
            Some(c) => Ok(c),
            None if let Some(ASTType {
                owner,
                content: ASTTypeKind::Component(comp),
            }) = queue.lowerer.lookup.find_type(symbol.owner, symbol.term) =>
            {
                let comp = queue.modules.get_entry(owner).component().get(comp);
                queue.enqueue_component(comp, owner)
            }
            None => {
                return Err(HIRError::component_not_found(
                    self.file(),
                    symbol.term,
                    span,
                ));
            }
        }
    }

    pub(crate) fn build_component_expression(
        &mut self,
        queue: &HirQueueBuilder,
        ComponentExpressionDescriptor {
            component,
            span,
            expected,
            context,
        }: ComponentExpressionDescriptor,
    ) -> Result<Spanned<PoolId<HirComponentExpression>>> {
        let lowered = self.self_type_or_lower(queue, component.name, context)?;
        let ty_view = queue.hir.view(lowered.term);
        let Some(comp_view) = ty_view.nominal().is_component() else {
            return Err(HIRError::not_a_component(self.file(), lowered.term, span));
        };

        let hircomponent = self.get_or_enqueue_component(
            queue,
            HirSymbol {
                owner: comp_view.owner(),
                term: comp_view.name(),
            },
            span,
        )?;

        let arity = queue
            .hir
            .get_file(hircomponent.owner)
            .declarations
            .components[hircomponent.term]
            .generics
            .len();

        // Explicit type arguments can come from one of two places: a literal
        // that spells them out (`Box<int> { .. }`, whose lowered type is an
        // `Apply`), or the type expected at this position (`let b: Box<int> =
        // Box { .. }`). Both are recovered before inference so explicit
        // arguments always win over the ones we infer from property values.
        let mut generics = {
            let explicit: Vec<TermId> = match ty_view.raw().node() {
                TermNode::Apply { args, .. } => args.clone(),
                _ => match expected {
                    Some(expected)
                        if queue
                            .hir
                            .view(expected)
                            .nominal()
                            .is_component()
                            .map(|c| c.data())
                            == Some(comp_view.data())
                            && let TermNode::Apply { args, .. } =
                                queue.hir.view(expected).raw().node() =>
                    {
                        args.clone()
                    }
                    _ => vec![],
                },
            };
            GenericTypeArguments { slots: explicit }
        };

        let mut properties = Vec::new();
        let has_declared_children = !comp_view.children().is_empty();
        let mut children: Vec<Option<Spanned<PoolId<HirComponentExpression>>>> =
            if has_declared_children {
                vec![None; comp_view.children().len()]
            } else {
                Vec::new()
            };
        for value in &component.values {
            match value {
                ComponentMemberValue::Assign { prop_name, rhs }
                    if let Some(position) = comp_view
                        .props()
                        .iter()
                        .position(|prop| prop.name == *prop_name) =>
                {
                    let declared_type = comp_view.props()[position].ty;

                    let expected_ty = match queue.hir.view(declared_type).raw().is_var_type() {
                        Some(var) => generics.get(var.index as usize),
                        None => Some(declared_type),
                    };
                    let expr = self.build_expression(
                        queue,
                        ExpressionDescriptor {
                            target: *rhs,
                            expected: expected_ty,
                            context,
                        },
                    )?;
                    self.unify_terms(
                        queue,
                        queue.hir.view(expr.data).ty().term,
                        declared_type,
                        span,
                    )?;
                    if let Some(var) = queue.hir.view(declared_type).raw().is_var_type() {
                        generics.try_set(var.index as usize, queue.hir.view(expr.data).ty().term);
                    }
                    properties.push(PropertyExpression::new(position, expr));
                }
                ComponentMemberValue::Assign { prop_name, .. } => {
                    // couldnt find any member with name `prop_name`
                    return Err(HIRError::property_unrecognized(
                        self.file(),
                        lowered.term,
                        vec![*prop_name],
                        span,
                    ));
                }
                ComponentMemberValue::Child(child) => {
                    if !has_declared_children {
                        let child_expr = self.build_component_expression(
                            queue,
                            ComponentExpressionDescriptor {
                                component: child,
                                span,
                                expected: None,
                                context,
                            },
                        )?;
                        children.push(Some(child_expr));
                        continue;
                    }
                    let position = children.iter().position(Option::is_none).ok_or_else(|| {
                        HIRError::new(self.file(), HIRErrorKind::InvalidChild, span)
                    })?;
                    let child_type =
                        generics.substitute(queue.hir, comp_view.children()[position].ty);
                    let child_expr = self.build_component_expression(
                        queue,
                        ComponentExpressionDescriptor {
                            component: child,
                            span,
                            expected: Some(child_type),
                            context,
                        },
                    )?;
                    let actual_type = queue.hir[child_expr.data].name;
                    self.unify_terms(queue, actual_type, child_type, span)?;
                    if let Some(var) = queue
                        .hir
                        .view(comp_view.children()[position].ty)
                        .raw()
                        .is_var_type()
                    {
                        generics.try_set(var.index as usize, actual_type);
                    }
                    children[position] = Some(child_expr);
                }
                ComponentMemberValue::NamedChild { name, child } => {
                    if !has_declared_children {
                        return Err(HIRError::new(self.file(), HIRErrorKind::InvalidChild, span));
                    }
                    let position = comp_view
                        .children()
                        .iter()
                        .position(|child| matches!(child.name, Some(cname) if cname == *name))
                        .ok_or_else(|| {
                            HIRError::new(self.file(), HIRErrorKind::InvalidChild, span)
                        })?;
                    if children[position].is_some() {
                        return Err(HIRError::already_defined(self.file(), *name, span));
                    }
                    let child_type =
                        generics.substitute(queue.hir, comp_view.children()[position].ty);
                    let child_expr = self.build_component_expression(
                        queue,
                        ComponentExpressionDescriptor {
                            component: child,
                            span,
                            expected: Some(child_type),
                            context,
                        },
                    )?;
                    let actual_type = queue.hir[child_expr.data].name;
                    self.unify_terms(queue, actual_type, child_type, span)?;
                    if let Some(var) = queue
                        .hir
                        .view(comp_view.children()[position].ty)
                        .raw()
                        .is_var_type()
                    {
                        generics.try_set(var.index as usize, actual_type);
                    }
                    children[position] = Some(child_expr);
                }
            }
        }

        // A generic component is only usable once every parameter has a
        // concrete argument; anything still unresolved cannot be specialized
        // and must be reported rather than silently left as a bare `Box` that
        // codegen cannot lower.
        if arity > 0 && (0..arity).any(|index| !generics.is_resolved(index)) {
            return Err(HIRError::couldnt_infer(self.file(), span));
        }
        let concrete = if arity > 0 {
            generics.finish_ref(queue.hir, lowered.term)
        } else {
            lowered.term
        };
        let children = children
            .into_iter()
            .enumerate()
            .filter_map(|(index, child)| {
                child.map(|child| ComponentChildExpression::new(index, child))
            })
            .collect();

        // Required component properties and inputs must be provided by the
        // caller. Requiredness is stored on the component type itself.
        let provided: Vec<usize> = properties.iter().map(|property| property.index()).collect();
        let mut missing = Vec::new();
        for (position, prop) in comp_view.props().iter().enumerate() {
            if prop.data.required && !provided.contains(&position) {
                missing.push(prop.data.name);
            }
        }
        if !missing.is_empty() {
            return Err(HIRError::missing_properties(self.file(), missing, span));
        }

        let id = queue
            .hir
            .store
            .insert_component_expression(HirComponentExpression {
                name: concrete,
                properties,
                children,
            });
        Ok(span.make_spanned(id))
    }
}
