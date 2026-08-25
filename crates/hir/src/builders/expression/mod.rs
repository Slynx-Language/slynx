use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use common::{
    Spanned,
    pool::{DedupPoolId, PoolId},
};
use module_loader::FileId;
use slynx_parser::{ASTExpression, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, HirStatement, HirType, Result, SymbolPointer,
    VariableId, builders::HirQueueBuilder, context::ScopeContext, id::OwnerId,
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MoveState {
    None,
    ///Used when a variable is moved by something that's not a variable (e.g. a function call).
    Moved,
    ///Used when a variable is moved by a variable (e.g. a let binding).
    MovedBy(SymbolPointer),
}

#[derive(Debug)]
pub enum BorrowState {
    Mutable,
    Immutable,
    Moved(MoveState),
}

#[derive(Debug)]
///This borrow state counts how much references there are to a variable. If 0 on both mutable and immutable, then its not referenced anywhere
pub struct VariableBorrowing {
    pub mutable: u8,
    pub immutable: u8,
    pub moved: MoveState,
}
impl VariableBorrowing {
    pub fn new() -> Self {
        Self {
            mutable: 0,
            immutable: 0,
            moved: MoveState::None,
        }
    }

    pub fn is_moved(&self) -> bool {
        self.moved != MoveState::None
    }

    ///Marks this borrowing as moved. If the given `by` is `None`, its moved normally, such as by a function call, but if it's `Some`, its moved by a variable.
    pub fn mark_moved(&mut self, by: Option<SymbolPointer>) {
        if let Some(by) = by {
            self.moved = MoveState::MovedBy(by);
        } else {
            self.moved = MoveState::Moved;
        }
    }

    ///Checks if the variable is referenced anywhere.
    pub fn is_referenced(&self) -> bool {
        self.mutable > 0 || self.immutable > 0
    }
    ///Checks if the variable is borrowed mutably.
    pub fn is_mutable(&self) -> bool {
        self.mutable > 0
    }
    ///Checks if the variable is borrowed immutably.
    pub fn is_immutable(&self) -> bool {
        self.immutable > 0
    }
    ///Borrows the variable mutably.
    pub fn borrow_mut(&mut self) {
        self.mutable += 1;
    }
    ///Borrows the variable immutably.
    pub fn borrow_immut(&mut self) {
        self.immutable += 1;
    }
    ///Releases a mutable borrow on the variable.
    pub fn release_mut(&mut self) {
        self.mutable = self.mutable.saturating_sub(1);
    }
    ///Releases an immutable borrow on the variable.
    pub fn release_immut(&mut self) {
        self.immutable = self.immutable.saturating_sub(1);
    }
    ///Checks if the variable can be borrowed mutably.
    pub fn can_mutable_borrow(&self) -> bool {
        self.mutable == 0 && self.immutable == 0
    }
    ///Checks if the variable can be borrowed immutably.
    pub fn can_immutable_borrow(&self) -> bool {
        self.mutable == 0
    }
}

#[derive(Debug)]
pub struct VariableInfo {
    pub name: SymbolPointer,
    pub state: VariableBorrowing,
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
}

impl ExpressionBuilder {
    pub fn new(owner: OwnerId) -> Self {
        Self {
            target: owner,
            variables: VariablesManager::default(),
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
                state: VariableBorrowing::new(),
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

    fn is_mutable(&self, id: VariableId) -> bool {
        self.variables
            .variables
            .get(&id)
            .map_or(false, |info| info.mutable)
    }

    fn borrowing(&self, id: VariableId) -> &VariableBorrowing {
        self.variables
            .variables
            .get(&id)
            .map(|info| &info.state)
            .expect("Variable should contains a borrowing state")
    }
    fn borrowing_mut(&mut self, id: VariableId) -> &mut VariableBorrowing {
        self.variables
            .variables
            .get_mut(&id)
            .map(|info| &mut info.state)
            .expect("Variable should contains a borrowing state")
    }
    pub(super) fn is_expression_able_to_write(
        &self,
        queue: &HirQueueBuilder,
        expr: Spanned<PoolId<HirExpression>>,
    ) -> Result<()> {
        let expression = &queue.hir[expr.data];
        match expression.kind {
            HirExpressionKind::Identifier(ident) => {
                match queue.hir.view(expr.data).ty_viewer().raw() {
                    HirType::MutableRef(_) => Ok(()),
                    _ if self.is_mutable(ident) && self.borrowing(ident).can_mutable_borrow() => {
                        Ok(())
                    }
                    _ if self.is_mutable(ident) && self.borrowing(ident).is_mutable() => {
                        let name = self.variable_name(ident).expect("Variable contain name");
                        Err(HIRError::borrowed_value(
                            name,
                            BorrowState::Mutable,
                            expr.span,
                        ))
                    }
                    _ => {
                        let name = self.variable_name(ident).expect(
                        "name of variable should be visible. Something is creating a variable on function builders, but for some reason not defining them on the builder names",
                    );
                        Err(HIRError::invalid_variable_write(name, expr.span))
                    }
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
        let exprid = queue.hir.insert_expression(expr);
        if let Some(id) = queue.hir.view(exprid).can_move() {
            self.borrowing_mut(id).mark_moved(None);
        }
        Ok(expression.span.make_spanned(exprid))
    }
}
