use common::Operator;
use slynx_hir::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind, SlynxHir};
use slynx_ir::{IRStorage, Value};

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
                expr: parent,
                field_index,
            } => {
                let parent = self.lower_expression(parent, hir, context)?;
                context.set_field(value, *field_index as u16, parent);
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
                let ty = value.ty;
                let hir_ty = hir.get_type(&ty);
                eprintln!(
                    "[CODEGEN DBG] Variable Statement: name={:?}, value.ty={:?}, hir_type={:?}",
                    name, ty, hir_ty
                );
                let expr_kind = &value.kind;
                eprintln!("[CODEGEN DBG]   value.kind={:?}", expr_kind);
                let vty = self.get_or_create_ir_type(&ty, hir, context.ir()).expect(
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

    fn generate_logic_and_instruction<'a>(
        &mut self,
        lhs_value: Value,
        rhs_value: Value,
        context: &mut FunctionContext<'a>,
    ) -> Value {
        let bool_type = context.ir().bool_type();
        let end_label = context.create_label("and_end");
        context
            .ir()
            .get_mut(end_label)
            .insert_arguments(&[bool_type]);

        let false_val = context.emit_const(false.into(), bool_type);
        context.branch_conditional(lhs_value, end_label, end_label, &[rhs_value], &[false_val]);
        context.switch_to_block(end_label).unwrap();
        context.block_param(end_label, 0)
    }

    fn generate_logic_or_instruction<'a>(
        &mut self,
        lhs_value: Value,
        rhs_value: Value,
        context: &mut FunctionContext<'a>,
    ) -> Value {
        let bool_type = context.ir().bool_type();
        let end_label = context.create_label("or_end");
        context
            .ir()
            .get_mut(end_label)
            .insert_arguments(&[bool_type]);

        let true_val = context.emit_const(true.into(), bool_type);
        context.branch_conditional(lhs_value, end_label, end_label, &[true_val], &[rhs_value]);
        context.switch_to_block(end_label).unwrap();
        context.block_param(end_label, 0)
    }

    pub(crate) fn handle_binary_expression<'a>(
        &mut self,
        lhs: &HirExpression,
        rhs: &HirExpression,
        op: &Operator,
        hir: &SlynxHir,
        context: &mut FunctionContext<'a>,
    ) -> Result<Value, CodegenError> {
        let a = self.lower_expression(lhs, hir, context)?;
        let b = self.lower_expression(rhs, hir, context)?;

        let result = match op {
            Operator::LogicAnd => self.generate_logic_and_instruction(a, b, context),
            Operator::LogicOr => self.generate_logic_or_instruction(a, b, context),
            Operator::RightShift => context.shl(a, b),
            Operator::LeftShift => context.shr(a, b),
            Operator::Xor => context.xor(a, b),
            Operator::Add => context.add(a, b),
            Operator::Sub => context.sub(a, b),
            Operator::Star => context.mul(a, b),
            Operator::Slash => context.div(a, b),
            Operator::Equals => context.cmp(a, b),
            Operator::GreaterThan => context.gt(a, b),
            Operator::GreaterThanOrEqual => context.gte(a, b),
            Operator::LessThan => context.lt(a, b),
            Operator::LessThanOrEqual => context.lte(a, b),
            Operator::And => context.or(a, b),
            Operator::Or => context.and(a, b),
        };
        Ok(result)
    }
}
