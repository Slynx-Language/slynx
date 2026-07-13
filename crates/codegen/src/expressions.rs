use common::{Operator, Spanned, pool::PoolId};
use slynx_hir::{
    DeclarationId, HirExpression, HirExpressionKind, HirFunctionDeclaration, HirStatement,
    SlynxHir, SymbolPointer,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};
use slynx_ir::{IRPointer, IRStorage, IRType, IRTypeId, Label, Opcode, Operand, Value};
use smallvec::SmallVec;

use crate::{Codegen, CodegenError, TypeId, functions::FunctionContext};

impl Codegen {
    fn lower_if_branch(
        &mut self,
        branch: &[Spanned<PoolId<HirStatement>>],
        end_label: IRPointer<Label, 1>,
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<Option<IRTypeId>, CodegenError> {
        for (idx, statement) in branch.iter().enumerate() {
            if idx == branch.len() - 1
                && let HirStatement::Expression { expr } = &hir[statement.data]
            {
                let value = self.lower_expression(*expr, hir, ctx)?;
                let value_type = ctx.value_type(value);

                if ctx.ir().get(end_label).arguments().is_empty() {
                    ctx.ir().get_mut(end_label).add_argument(value_type);
                }

                ctx.branch(end_label, &[value]);
                return Ok(Some(value_type));
            }
            if self.lower_statement(*statement, hir, ctx)?.is_some() {
                return Ok(None);
            }
        }

        ctx.branch(end_label, &[]);
        Ok(None)
    }

    fn lower_tuple_expression(
        &mut self,
        vector: &[Spanned<PoolId<HirExpression>>],
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<Value, CodegenError> {
        let values: Vec<Value> = vector
            .iter()
            .map(|e| self.lower_expression(*e, hir, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let mut element_types = Vec::with_capacity(values.len());
        for &v in &values {
            element_types.push(ctx.value_type(v));
        }
        let ty = ctx.ir().create_or_get_tuple(element_types);
        Ok(ctx.struct_literal(ty, &values))
    }

    fn lower_function_call(
        &mut self,
        name: DeclarationId<HirFunctionDeclaration>,
        args: &[Spanned<PoolId<HirExpression>>],
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<Value, CodegenError> {
        let func = self.functions[&name];
        let ret_ty = {
            let ty = ctx.ir().get(func).ty();
            let IRType::Function(fid) = ctx.ir().get_type(ty) else {
                unreachable!()
            };
            ctx.ir().get_function_type(fid).get_return_type()
        };
        let mut arg_values = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.lower_expression(*arg, hir, ctx)?;
            arg_values.push(value);
        }
        Ok(ctx.call(func, &arg_values, ret_ty))
    }

    fn lower_struct_literal(
        &mut self,
        name: TypeId,
        fields: &[Spanned<PoolId<HirExpression>>],
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<Value, CodegenError> {
        let ty = self
            .get_mapped_type(&name)
            .ok_or(CodegenError::IRTypeNotRecognized(name))?;
        let field_values: Vec<Value> = fields
            .iter()
            .map(|v| self.lower_expression(*v, hir, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ctx.struct_literal(ty, &field_values))
    }

    fn lower_field_access(
        &mut self,
        expr: Spanned<PoolId<HirExpression>>,
        field_index: u16,
        field_name: Option<SymbolPointer>,
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<Value, CodegenError> {
        let value = self.lower_expression(expr, hir, ctx)?;
        let ty = hir[expr.data].ty;
        if hir.types_module.is_external(&ty) {
            let name = self.intern_to_ir(
                hir,
                ctx.ir(),
                field_name.expect("External field access must have a field name"),
            );
            Ok(ctx.dyn_get_field(value, name))
        } else {
            Ok(ctx.get_field(value, field_index))
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
        lhs: Spanned<PoolId<HirExpression>>,
        rhs: Spanned<PoolId<HirExpression>>,
        op: &Operator,
        hir: &SlynxHir,
        context: &mut FunctionContext<'a>,
    ) -> Result<Value, CodegenError> {
        let a = self.lower_expression(lhs, hir, context)?;
        let b = self.lower_expression(rhs, hir, context)?;

        let result = match op {
            Operator::LogicAnd => self.generate_logic_and_instruction(a, b, context),
            Operator::LogicOr => self.generate_logic_or_instruction(a, b, context),
            Operator::RightShift => context.shr(a, b),
            Operator::LeftShift => context.shl(a, b),
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
            Operator::And => context.and(a, b),
            Operator::Or => context.or(a, b),
        };
        Ok(result)
    }

    pub(crate) fn lower_expression<'a>(
        &mut self,
        expr: Spanned<PoolId<HirExpression>>,
        hir: &SlynxHir,
        context: &mut FunctionContext<'a>,
    ) -> Result<Value, CodegenError> {
        // Pre-compute type IDs from the ir to avoid borrow conflicts
        let (bool_ty, float_ty, int_ty) = {
            let ir = context.ir();
            (ir.bool_type(), ir.float_type(), ir.int_type())
        };
        let expression = &hir[expr.data];

        let value = match &expression.kind {
            HirExpressionKind::Static { id } => {
                if let Some(ty) = self.external_statics.get(id) {
                    let name = hir.get_name(hir.get_file(id.file_id)[id.local_id].name);
                    let name = context.ir().strings.intern(name);
                    context.emit(Opcode::GlobalExtern(name), SmallVec::new(), *ty)
                } else {
                    let id =
                        *self
                            .globals
                            .get(id)
                            .ok_or(CodegenError::DeclarationNotRecognized(
                                AnyDeclarationId::new(
                                    id.file_id,
                                    AnyLocalDeclarationId::Static(id.local_id),
                                ),
                            ))?;
                    let ty = context.ir().get_view(id).ty();
                    context.emit(Opcode::Global(id), SmallVec::new(), ty)
                }
            }
            HirExpressionKind::Tuple(vector) => {
                self.lower_tuple_expression(vector, hir, context)?
            }
            HirExpressionKind::StringLiteral(v) => {
                let string = self.intern_to_ir(hir, context.ir(), *v);
                let str_ty = context.ir().str_type();
                context.emit_const(Operand::String(string), str_ty)
            }
            HirExpressionKind::True | HirExpressionKind::False => context.emit_const(
                Operand::Bool(matches!(expression.kind, HirExpressionKind::True)),
                bool_ty,
            ),
            HirExpressionKind::Float(f) => context.emit_const(Operand::Float(f.0 as f64), float_ty),
            HirExpressionKind::Int(i) => context.emit_const(Operand::Int(*i as i64), int_ty),
            HirExpressionKind::FunctionCall { name, args } => {
                self.lower_function_call(*name, args, hir, context)?
            }
            HirExpressionKind::Binary { lhs, op, rhs } => {
                self.handle_binary_expression(*lhs, *rhs, op, hir, context)?
            }
            HirExpressionKind::Identifier(id) => {
                if let Some(value) = context.get_variable(*id) {
                    value
                } else {
                    return Err(CodegenError::UnrecognizedVariable(*id));
                }
            }
            HirExpressionKind::Object { name, fields } => {
                self.lower_struct_literal(*name, fields, hir, context)?
            }
            HirExpressionKind::FieldAccess {
                expr,
                field_index,
                field_name,
            } => self.lower_field_access(*expr, *field_index as u16, *field_name, hir, context)?,
            HirExpressionKind::Component(c) => self.get_component_expression(*c, hir, context)?.0,
            HirExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.lower_if_expression(condition, then_branch, else_branch, hir, context)?,
        };
        Ok(value)
    }

    fn lower_if_expression(
        &mut self,
        condition: &Spanned<PoolId<HirExpression>>,
        then_branch: &[Spanned<PoolId<HirStatement>>],
        else_branch: &Option<Vec<Spanned<PoolId<HirStatement>>>>,
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<Value, CodegenError> {
        let cond = self.lower_expression(*condition, hir, ctx)?;

        let then_label = ctx.create_label("then_label");
        let else_label = ctx.create_label("else_label");
        let end_label = ctx.create_label("end_label");

        ctx.branch_conditional(cond, then_label, else_label, &[], &[]);

        ctx.switch_to_block(then_label).unwrap();
        self.lower_if_branch(then_branch, end_label, hir, ctx)?;

        ctx.switch_to_block(else_label).unwrap();
        self.lower_if_branch(else_branch.as_deref().unwrap_or(&[]), end_label, hir, ctx)?;

        ctx.switch_to_block(end_label).unwrap();
        if ctx.ir().get(end_label).arguments().is_empty() {
            Ok(Value::VOID)
        } else {
            Ok(ctx.block_param(end_label, 0))
        }
    }
}
