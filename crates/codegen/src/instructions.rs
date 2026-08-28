use common::{Spanned, pool::PoolId};
use slynx_hir::{HirExpression, HirExpressionKind, HirStatement, SlynxHir};
use slynx_ir::{Opcode, Value};
use smallvec::smallvec;

use crate::{Codegen, CodegenError, functions::FunctionContext};

impl Codegen {
    fn emit_while_statement<'a>(
        &mut self,
        condition: &Spanned<PoolId<HirExpression>>,
        body: &[Spanned<PoolId<HirStatement>>],
        hir: &SlynxHir,
        context: &mut FunctionContext<'a>,
    ) -> Result<(), CodegenError> {
        let cond_label = context.create_label("while_cond");
        let body_label = context.create_label("while_body");
        let end_label = context.create_label("while_end");

        context.switch_to_block(cond_label).unwrap();
        let cond_value = self.lower_expression(*condition, hir, context)?;
        context.branch_conditional(cond_value, body_label, end_label, &[], &[]);

        context.switch_to_block(body_label).unwrap();
        for stmt in body {
            self.lower_statement(*stmt, hir, context)?;
        }
        context.branch(cond_label, &[]);

        context.switch_to_block(end_label).unwrap();
        Ok(())
    }

    fn emit_assign_statement<'a>(
        &mut self,
        lhs: Spanned<PoolId<HirExpression>>,
        value: Spanned<PoolId<HirExpression>>,
        hir: &SlynxHir,
        context: &mut FunctionContext<'a>,
    ) -> Result<(), CodegenError> {
        let value = self.lower_expression(value, hir, context)?;
        let lhs_raw = &hir[lhs.data];
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
            } if let HirExpressionKind::Deref(inner) = hir.expressions[expr.data].kind => {
                let field_type = {
                    let field_type = {
                        if let Some(ty) = hir.view(inner.data).ty_viewer().is_mutable_ref()
                            && let Some(s) = ty.is_struct()
                        {
                            s.field_types()[*field_index]
                        } else {
                            panic!(
                                "This shit should be a reference type, and since its inside a field access, a reference to a struct"
                            )
                        }
                    };
                    let ty = self.get_or_create_ir_type(&field_type, hir, context.ir())?;
                    context.ir().pointer_type(ty)
                };
                let parent = self.lower_expression(inner, hir, context)?;
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
                let is_external = hir.types_module.is_external(&hir[parent_expr.data].ty);

                let parent = self.lower_expression(*parent_expr, hir, context)?;
                match is_external {
                    true => {
                        let name = self.intern_to_ir(
                            hir,
                            context.ir(),
                            field_name.expect("External field access must have a field name"),
                        );
                        context.dyn_set_field(parent, name, value)
                    }
                    _ => context.set_field(parent, *field_index as u16, value),
                };
            }
            HirExpressionKind::Deref(parent_expr) => {
                let parent = self.lower_expression(*parent_expr, hir, context)?;
                let ty = context.ir().value_type(value);
                context.emit(Opcode::DerefWrite, smallvec![parent, value], ty);
            }
            recv => unreachable!(
                "LHS of assignment must be Identifier, FieldAccess or a deref, received {recv:?}"
            ),
        }
        Ok(())
    }

    pub(crate) fn lower_statement<'a>(
        &mut self,
        statement: Spanned<PoolId<HirStatement>>,
        hir: &SlynxHir,
        context: &mut FunctionContext<'a>,
    ) -> Result<Option<Value>, CodegenError> {
        let stmt = &hir[statement.data];
        match &stmt {
            HirStatement::While { condition, body } => {
                self.emit_while_statement(condition, body, hir, context)?;
                Ok(None)
            }
            HirStatement::Variable { name, value } => {
                let vty = self.get_or_create_ir_type(&hir[value.data].ty, hir, context.ir()).expect(
                    "Type of variable creation should be hoisted before mapping function bodies",
                );
                let slot = context.allocate(vty);
                let val = self.lower_expression(*value, hir, context)?;
                context.write(slot, val);
                context.add_variable(*name, slot);
                Ok(None)
            }
            HirStatement::Assign { lhs, value } => {
                self.emit_assign_statement(*lhs, *value, hir, context)?;
                Ok(None)
            }
            HirStatement::Expression { expr } => {
                let value = self.lower_expression(*expr, hir, context)?;
                Ok(Some(value))
            }
            HirStatement::Return { expr } => {
                if let Some(expr) = expr {
                    let value = self.lower_expression(*expr, hir, context)?;
                    context.ret(value);
                }
                Ok(None)
            }
        }
    }
}
