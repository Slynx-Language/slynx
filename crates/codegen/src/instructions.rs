use slynx_hir::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind, SlynxHir};
use slynx_ir::Value;

use crate::{Codegen, CodegenError, functions::FunctionContext};

impl Codegen {
    fn emit_while_statement<'a>(
        &mut self,
        condition: &HirExpression,
        body: &[HirStatement],
        hir: &SlynxHir,
        context: &mut FunctionContext<'a>,
    ) -> Result<(), CodegenError> {
        let cond_label = context.create_label("while_cond");
        let body_label = context.create_label("while_body");
        let end_label = context.create_label("while_end");

        context.switch_to_block(cond_label).unwrap();
        let cond_value = self.lower_expression(condition, hir, context)?;
        context.branch_conditional(cond_value, body_label, end_label, &[], &[]);

        context.switch_to_block(body_label).unwrap();
        for stmt in body {
            self.lower_statement(stmt, hir, context)?;
        }
        context.branch(cond_label, &[]);

        context.switch_to_block(end_label).unwrap();
        Ok(())
    }

    fn emit_assign_statement<'a>(
        &mut self,
        lhs: &HirExpression,
        value: &HirExpression,
        hir: &SlynxHir,
        context: &mut FunctionContext<'a>,
    ) -> Result<Option<Value>, CodegenError> {
        let value = self.lower_expression(value, hir, context)?;

        match &lhs.kind {
            HirExpressionKind::Identifier(id) => {
                let slot = context
                    .get_variable(*id)
                    .expect("Variable not found for assignment");
                context.write(slot, value);
            }
            HirExpressionKind::FieldAccess {
                expr: parent_expr,
                field_index,
                field_name,
            } => {
                let is_external = hir.types_module.is_external(&parent_expr.ty);
                let parent = self.lower_expression(parent_expr, hir, context)?;
                if is_external {
                    let name = self.intern_to_ir(hir, context.ir(), field_name.expect("External field access must have a field name"));
                    context.dyn_set_field(parent, name, value);
                } else {
                    context.set_field(parent, *field_index as u16, value);
                }
            }
            _ => unreachable!("LHS of assignment must be Identifier or FieldAccess"),
        }
        Ok(None)
    }

    pub(crate) fn lower_statement<'a>(
        &mut self,
        statement: &HirStatement,
        hir: &SlynxHir,
        context: &mut FunctionContext<'a>,
    ) -> Result<Option<Value>, CodegenError> {
        match &statement.kind {
            HirStatementKind::While { condition, body } => {
                self.emit_while_statement(condition, body, hir, context)?;
                Ok(None)
            }
            HirStatementKind::Variable { name, value } => {
                let vty = self.get_or_create_ir_type(&value.ty, hir, context.ir()).expect(
                    "Type of variable creation should be hoisted before mapping function bodies",
                );
                let slot = context.allocate(vty);
                let val = self.lower_expression(value, hir, context)?;
                context.write(slot, val);
                context.add_variable(*name, slot);
                Ok(None)
            }
            HirStatementKind::Assign { lhs, value } => {
                self.emit_assign_statement(lhs, value, hir, context)
            }
            HirStatementKind::Expression { expr } => {
                let value = self.lower_expression(expr, hir, context)?;
                Ok(Some(value))
            }
            HirStatementKind::Return { expr } => {
                let value = self.lower_expression(expr, hir, context)?;
                context.ret(value);
                Ok(None)
            }
        }
    }
}
