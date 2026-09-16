use common::{Spanned, pool::PoolId};
use slynx_hir::{HirExpressionKind, HirStatement};
use slynx_ir::{Opcode, Value};
use smallvec::smallvec;

use crate::{
    CodegenError,
    lowerers::{LoweringState, functions::FunctionContext},
};

impl<'a> LoweringState<'a> {
    fn emit_while_statement<'b>(
        &mut self,
        condition: &Spanned<PoolId<slynx_hir::HirExpression>>,
        body: &[Spanned<PoolId<HirStatement>>],
        context: &mut FunctionContext<'b>,
    ) -> Result<(), CodegenError> {
        let cond_label = context.create_label("while_cond");
        let body_label = context.create_label("while_body");
        let end_label = context.create_label("while_end");

        context.switch_to_block(cond_label).unwrap();
        let cond_value = self.lower_expression(*condition, context)?;
        context.branch_conditional(cond_value, body_label, end_label, &[], &[]);

        context.switch_to_block(body_label).unwrap();
        for stmt in body {
            self.lower_statement(*stmt, context)?;
        }
        context.branch(cond_label, &[]);

        context.switch_to_block(end_label).unwrap();
        Ok(())
    }

    fn emit_assign_statement<'b>(
        &mut self,
        lhs: Spanned<PoolId<slynx_hir::HirExpression>>,
        value: Spanned<PoolId<slynx_hir::HirExpression>>,
        context: &mut FunctionContext<'b>,
    ) -> Result<(), CodegenError> {
        let value = self.lower_expression(value, context)?;
        let lhs_raw = &self.hir[lhs.data];
        match &lhs_raw.kind {
            HirExpressionKind::Identifier(id) => {
                let slot = context
                    .get_variable(*id)
                    .expect("Variable not found for assignment");
                context.write(slot, value);
            }
            HirExpressionKind::FieldAccess {
                expr,
                field_index,
                field_name,
            } if let HirExpressionKind::Deref(inner) = self.hir.store.expressions[expr.data].kind => {
                let field_type =
                    self.types.deref_field_type(inner.data, *field_index, context.ir())?;
                let parent = self.lower_expression(inner, context)?;
                let parent = context.emit(
                    Opcode::FieldRef(*field_index as u16),
                    smallvec![parent],
                    field_type,
                );
                context.deref_write(parent, value);
            }
            HirExpressionKind::FieldAccess {
                expr: parent_expr,
                field_index,
                field_name,
            } => {
                let is_external = self.hir.types.is_external(&self.hir[parent_expr.data].ty);

                let parent = self.lower_expression(*parent_expr, context)?;
                match is_external {
                    true => {
                        let name = self.intern_to_ir(
                            context.ir(),
                            field_name.expect("External field access must have a field name"),
                        );
                        context.dyn_set_field(parent, name, value)
                    }
                    _ => context.set_field(parent, *field_index as u16, value),
                };
            }
            HirExpressionKind::Deref(parent_expr) => {
                let parent = self.lower_expression(*parent_expr, context)?;
                let ty = context.ir().value_type(value);
                context.emit(Opcode::DerefWrite, smallvec![parent, value], ty);
            }
            recv => unreachable!(
                "LHS of assignment must be Identifier, FieldAccess or a deref, received {recv:?}"
            ),
        }
        Ok(())
    }

    pub(crate) fn lower_statement<'b>(
        &mut self,
        statement: Spanned<PoolId<HirStatement>>,
        context: &mut FunctionContext<'b>,
    ) -> Result<Option<Value>, CodegenError> {
        let stmt = &self.hir[statement.data];
        match &stmt {
            HirStatement::While { condition, body } => {
                self.emit_while_statement(condition, body, context)?;
                Ok(None)
            }
            HirStatement::Variable { name, value } => {
                let vty = self.types.get_or_create_ir_type(
                    self.hir[value.data].ty,
                    context.ir(),
                ).expect(
                    "Type of variable creation should be hoisted before mapping function bodies",
                );
                let slot = context.allocate(vty);
                let val = self.lower_expression(*value, context)?;
                context.write(slot, val);
                context.add_variable(*name, slot);
                Ok(None)
            }
            HirStatement::Assign { lhs, value } => {
                self.emit_assign_statement(*lhs, *value, context)?;
                Ok(None)
            }
            HirStatement::Expression { expr } => {
                let value = self.lower_expression(*expr, context)?;
                Ok(Some(value))
            }
            HirStatement::Return { expr } => {
                if let Some(expr) = expr {
                    let value = self.lower_expression(*expr, context)?;
                    context.ret(value);
                }
                Ok(None)
            }
        }
    }
}