use std::collections::{HashMap, HashSet};

use common::{
    Spanned,
    pool::{DedupPoolId, PoolId},
};
use module_loader::FileId;
use slynx_parser::{ASTExpression, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, HirStatement, HirType, Result, SymbolPointer,
    VariableId, builders::HirQueueBuilder, id::OwnerId,
};

mod calls;
mod collections;
mod components;
mod control_flow;
mod field_access;
mod literals;
mod names;
mod objects;
mod statements;
mod typing;

/// Result of building a body with the ExpressionBuilder.
pub(crate) struct ExpressionBuildResult {
    pub(crate) args: Vec<VariableId>,
    pub(crate) statements: Vec<Spanned<PoolId<HirStatement>>>,
}

/// A single, reusable expression builder that can be used by both function
/// builders and component builders. Owns the state required for expression
/// generation (variables, type mappings, etc.).
pub(crate) struct ExpressionBuilder {
    pub(crate) target: OwnerId,
    pub(crate) names: HashMap<SymbolPointer, VariableId>,
    pub(crate) variables_types: HashMap<VariableId, DedupPoolId<HirType>>,
    pub(crate) mutable: HashSet<VariableId>,
}

impl ExpressionBuilder {
    pub fn new(owner: OwnerId) -> Self {
        Self {
            target: owner,
            names: HashMap::new(),
            variables_types: HashMap::new(),
            mutable: HashSet::new(),
        }
    }

    pub fn file(&self) -> FileId {
        match self.target {
            OwnerId::Component(c) => c.file_id,
            OwnerId::Function(f) => f.file_id,
        }
    }

    pub fn create_mapped_variable(
        &mut self,
        name: SymbolPointer,
        id: VariableId,
        mutable: bool,
        ty: DedupPoolId<HirType>,
    ) {
        self.names.insert(name, id);
        self.variables_types.insert(id, ty);
        if mutable {
            self.mutable.insert(id);
        }
    }

    pub fn create_variable(
        &mut self,
        name: SymbolPointer,
        mutable: bool,
        ty: DedupPoolId<HirType>,
    ) -> VariableId {
        let id = VariableId::new(self.target, self.names.len() as u16);
        self.create_mapped_variable(name, id, mutable, ty);
        id
    }

    fn is_mutable(&self, id: VariableId) -> bool {
        self.mutable.contains(&id)
    }

    pub(super) fn is_expression_able_to_write(
        &self,
        queue: &HirQueueBuilder,
        expr: Spanned<PoolId<HirExpression>>,
    ) -> Result<()> {
        let expression = &queue.hir[expr.data];
        match expression.kind {
            HirExpressionKind::Identifier(ident) => {
                if self.is_mutable(ident) {
                    Ok(())
                } else if let HirType::MutableRef(_) = queue.hir.view(expr.data).ty_viewer().raw() {
                    Ok(())
                } else {
                    let ident = self
                        .names
                        .iter()
                        .find_map(|entry| (*entry.1 == ident).then_some(*entry.0))
                        .expect(
                            "name of variable should be visible. Something is creating a variable on function builders, but for some reason not defining them on the builder names",
                        );
                    Err(HIRError::invalid_variable_write(ident, expr.span))
                }
            }

            HirExpressionKind::FieldAccess { expr, .. } => {
                self.is_expression_able_to_write(queue, expr)
            }
            HirExpressionKind::Deref(inner)
                if let HirType::MutableRef(_) = queue.hir.view(inner.data).ty_viewer().raw() =>
            {
                Ok(())
            }
            HirExpressionKind::Deref(_) => Err(HIRError::invalid_ref_write(expr.span)),
            _ => Err(HIRError::invalid_expr_write(expr.span)),
        }
    }
}

impl ExpressionBuilder {
    pub(crate) fn build_expression(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        expression: Spanned<DedupPoolId<ASTExpression>>,
        expected: Option<DedupPoolId<HirType>>,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        let expr = queue.get_expr(expression.data);
        let expr = match expr {
            ASTExpression::Deref(expr) => self.build_deref(queue, *expr, expected, context)?,
            ASTExpression::Reference { mutable, expr } => {
                self.build_reference(queue, *expr, *mutable, expected, context)?
            }
            ASTExpression::Null => self.build_null(queue, expression.span, expected)?,
            ASTExpression::IndexExpression(expr, range) => {
                self.build_index(queue, *expr, range, expected, expression.span, context)?
            }
            ASTExpression::False => self.build_bool(queue, false),
            ASTExpression::True => self.build_bool(queue, true),
            ASTExpression::Identifier(name) => {
                self.build_identifier(queue, *name, expression.span)?
            }
            ASTExpression::IntLiteral(i) => self.build_int_literal(queue, *i),
            ASTExpression::FloatLiteral(f) => self.build_float_literal(queue, f.into_inner()),
            ASTExpression::StringLiteral(s) => self.build_str_literal(queue, *s),
            ASTExpression::Tuple(fields) => {
                self.build_tuple_expression(queue, fields, expected, context)?
            }

            ASTExpression::FieldAccess { parent, field } => {
                let parent = self.build_expression(queue, *parent, expected, context)?;
                return self.build_field_access(queue, parent, *field, expression.span, context);
            }
            ASTExpression::TupleAccess { tuple, index } => self.build_tuple_access(
                queue,
                *tuple,
                expected,
                expression.span,
                *index as usize,
                context,
            )?,
            ASTExpression::Binary { lhs, op, rhs } => {
                let lhs = self.build_expression(queue, *lhs, expected, context)?;
                let rhs = self.build_expression(queue, *rhs, expected, context)?;
                let lhs_ty = queue.hir.view(lhs.data).ty();
                let rhs_ty = queue.hir.view(rhs.data).ty();
                let ty = self.unify_types(queue, lhs_ty, rhs_ty, expression.span)?;
                let ty = if op.is_logical() {
                    queue.hir.create_type(HirType::Bool)
                } else {
                    ty
                };
                queue.hir.create_binary_expression(lhs, rhs, *op, ty)
            }
            ASTExpression::FunctionCall { name, args } => {
                self.build_function_call(queue, *name, args, context)?
            }
            ASTExpression::If {
                condition,
                body,
                else_body,
            } => self.build_if(
                queue,
                *condition,
                body,
                else_body,
                expression.span,
                expected,
                context,
            )?,
            ASTExpression::Component(component) => {
                let child =
                    self.build_component_expression(queue, component, expression.span, context)?;
                HirExpression {
                    ty: queue.hir[child.data].name,
                    kind: HirExpressionKind::Component(child),
                }
            }
            ASTExpression::ObjectExpression { name, fields } => {
                self.build_object(queue, *name, fields, expression.span, expected, context)?
            }
            ASTExpression::Array(expressions) => {
                self.build_array(queue, expressions, expression.span, expected, context)?
            }
            ASTExpression::Vector(expressions) => {
                self.build_vector(queue, expressions, expression.span, expected, context)?
            }
        };
        Ok(expression
            .span
            .make_spanned(queue.hir.insert_expression(expr)))
    }
}
