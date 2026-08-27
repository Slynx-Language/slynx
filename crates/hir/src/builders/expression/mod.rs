use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use common::{
    Spanned,
    pool::{DedupPoolId, PoolId},
};
use either::Either;
use module_loader::FileId;
use slynx_parser::{ASTExpression, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, HirStatement, HirType, Result, SymbolPointer,
    VariableId,
    builders::{
        HirQueueBuilder,
        expression::literals::{DereferenceExpressionDescriptor, ReferenceExpressionDescriptor},
    },
    context::ScopeContext,
    id::OwnerId,
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

#[derive(Debug)]
pub struct VariableInfo {
    pub name: SymbolPointer,
    pub type_id: DedupPoolId<HirType>,
    pub mutable: bool,
}

#[derive(Debug, Default)]
pub struct VariablesManager {
    pub scope: ScopeContext,
    pub variables: HashMap<VariableId, VariableInfo>,
}
impl Deref for VariablesManager {
    type Target = HashMap<VariableId, VariableInfo>;
    fn deref(&self) -> &Self::Target {
        &self.variables
    }
}
impl DerefMut for VariablesManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.variables
    }
}

/// A single, reusable expression builder that can be used by both function
/// builders and component builders. Owns the state required for expression
/// generation (variables, type mappings, etc.).
pub(crate) struct ExpressionBuilder {
    pub(crate) target: OwnerId,
    pub(crate) variables: VariablesManager,
    pub(crate) self_type: Option<DedupPoolId<HirType>>,
}

impl ExpressionBuilder {
    pub fn new(owner: OwnerId, self_type: Option<DedupPoolId<HirType>>) -> Self {
        Self {
            target: owner,
            variables: VariablesManager::default(),
            self_type,
        }
    }

    pub fn file(&self) -> FileId {
        match self.target {
            OwnerId::Component(c) => c.file_id,
            OwnerId::Function(f) => f.file_id,
        }
    }

    /// Finds the name of the variable with the given id. Note that this is a linear search, so it may be slow for bodies.
    /// This is intended mainly because bodies shouldn't have many variables, and the performance impact is negligible.
    pub fn variable_name(&self, id: VariableId) -> Option<SymbolPointer> {
        self.variables
            .scope
            .iter()
            .find_map(|scope| scope.contains(id))
    }

    pub fn create_mapped_variable(
        &mut self,
        name: SymbolPointer,
        id: VariableId,
        mutable: bool,
        ty: DedupPoolId<HirType>,
    ) {
        self.variables.scope.create_name(name, id, mutable);
        self.variables.insert(
            id,
            VariableInfo {
                name,
                type_id: ty,
                mutable,
            },
        );
    }

    pub fn create_variable(
        &mut self,
        name: SymbolPointer,
        mutable: bool,
        ty: DedupPoolId<HirType>,
    ) -> VariableId {
        let id = VariableId::new(self.target, self.variables.scope.variable_count() as u16);
        self.create_mapped_variable(name, id, mutable, ty);
        id
    }

    pub(super) fn is_expression_able_to_write(
        &self,
        queue: &HirQueueBuilder,
        expr: Spanned<PoolId<HirExpression>>,
    ) -> Result<()> {
        let expression = &queue.hir[expr.data];
        match expression.kind {
            HirExpressionKind::Identifier(ident) => {
                if let HirType::MutableRef(_) = queue.hir.view(expr.data).ty_viewer().raw() {
                    return Ok(());
                }
                if self
                    .variables
                    .variables
                    .get(&ident)
                    .map_or(false, |info| info.mutable)
                {
                    Ok(())
                } else {
                    let name = self.variable_name(ident).expect(
                        "name of variable should be visible. Something is creating a variable on function builders, but for some reason not defining them on the builder names",
                    );
                    Err(HIRError::invalid_variable_write(name, expr.span))
                }
            }

            HirExpressionKind::FieldAccess { expr, .. } => {
                self.is_expression_able_to_write(queue, expr)
            }
            HirExpressionKind::Deref(inner)
                if queue
                    .hir
                    .view(inner.data)
                    .ty_viewer()
                    .is_mutable_ref()
                    .is_some() =>
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
            ASTExpression::Deref(expr) => self.build_deref(
                queue,
                DereferenceExpressionDescriptor {
                    target: *expr,
                    context,
                    expected,
                },
            )?,
            ASTExpression::Reference { mutable, expr } => {
                return self.build_reference_expression(
                    queue,
                    ReferenceExpressionDescriptor {
                        target: Either::Left(*expr),
                        mutable: *mutable,
                        context,
                    },
                );
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
                return self.build_access(
                    queue,
                    *parent,
                    *field,
                    expected,
                    expression.span,
                    context,
                );
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
                self.build_function_call(queue, *name, args, context, expression.span)?
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
        let exprid = queue.hir.insert_expression(expr);
        Ok(expression.span.make_spanned(exprid))
    }
}
