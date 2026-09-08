use common::{Operator, Spanned, pool::PoolId};
use slynx_hir::{
    DeclarationId, HirExpression, HirExpressionKind, HirFunctionDeclaration, HirStatement, HirType,
    SlynxHir, SymbolPointer,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
    ownership::ExpressionUse,
};
use slynx_ir::{IRPointer, IRStorage, IRType, IRTypeId, Label, Opcode, Operand, Value};
use smallvec::{SmallVec, smallvec};

use crate::{Codegen, CodegenError, TypeId, functions::FunctionContext};

impl Codegen {
    fn lower_enum(
        &mut self,
        context: &mut FunctionContext,
        hir: &SlynxHir,
        ty: TypeId,
        variant: usize,
        args: &[Spanned<PoolId<HirExpression>>],
    ) -> Result<Value, CodegenError> {
        // Read the (post-monomorphization) enum type to find the variant's
        // compile-time discriminant. Enums lower to a struct whose field[0]
        // holds the discriminant tag and whose field[1] is a union of the
        // per-variant payload structs. Construction fills the tag plus the
        // selected variant's payload struct inside that union, using the
        // centralized `EnumLayout` so construction and matching agree on the
        // exact shape registered at materialization time.
        let view = hir.view(ty);
        let deref = view.dereference();
        let key = deref.data();
        let enum_view = deref.is_enum().ok_or_else(|| {
            CodegenError::InternalError("enum expression must resolve to an enum type".into())
        })?;
        let variant_info = enum_view.variants().get(variant).ok_or_else(|| {
            CodegenError::InternalError("enum variant index out of bounds".into())
        })?;

        let layout = self
            .enum_layouts
            .get(&key)
            .ok_or_else(|| {
                CodegenError::InternalError("enum layout is not registered".into())
            })?
            .clone();

        let int_type = context.ir().int_type();
        let tag = context.emit_const(Operand::Int(variant_info.discriminant as i64), int_type);

        let mut operands = Vec::with_capacity(2);
        operands.push(tag);

        if let Some(union_ty) = layout.union_type {
            let payload_struct = layout
                .variant_payload
                .get(variant)
                .copied()
                .flatten()
                .ok_or_else(|| {
                    CodegenError::InternalError(
                        "variant payload struct is not registered".into(),
                    )
                })?;
            let args = args
                .iter()
                .map(|arg| self.lower_expression(*arg, hir, context))
                .collect::<Result<Vec<_>, _>>()?;
            let payload = context.struct_literal(payload_struct, &args);
            let union_value = context.struct_literal(union_ty, std::slice::from_ref(&payload));
            operands.push(union_value);
        }

        Ok(context.struct_literal(layout.type_id, &operands))
    }

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
            let fid = *fid;
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

        match () {
            _ if hir.types_module.is_external(&ty) => {
                let name = self.intern_to_ir(
                    hir,
                    ctx.ir(),
                    field_name.expect("External field access must have a field name"),
                );
                Ok(ctx.dyn_get_field(value, name))
            }
            _ if let HirExpressionKind::Deref(inner) = hir.expressions[expr.data].kind => {
                let viewer = hir.view(inner.data);
                let type_viewer = viewer.ty_viewer();
                let concrete_type = type_viewer.concrete_type();
                let concrete_type = concrete_type
                    .is_struct()
                    .expect("Field access should be made on a struct type");
                let field_type = {
                    let tmp = concrete_type.field_types()[field_index as usize];
                    let field_type = self.get_or_create_ir_type(&tmp, hir, ctx.ir())?;
                    ctx.ir().pointer_type(field_type)
                };
                let base = self.lower_expression(inner, hir, ctx)?;

                let fp = ctx.field_ref(base, field_index);
                Ok(ctx.emit(Opcode::Deref, smallvec![fp], field_type))
            }
            _ => Ok(ctx.get_field(value, field_index)),
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
            HirExpressionKind::Deref(inner) => {
                let inner = self.lower_expression(*inner, hir, context)?;
                let ty = self.get_or_create_ir_type(&expression.ty, hir, context.ir())?;
                context.emit(Opcode::Deref, smallvec![inner], ty)
            }
            HirExpressionKind::Reference(inner) => {
                let inner = self.lower_expression(*inner, hir, context)?;
                let ty = self.get_or_create_ir_type(&expression.ty, hir, context.ir())?;
                context.emit(Opcode::Ref, smallvec![inner], ty)
            }
            HirExpressionKind::Null => {
                let HirType::Nullable(inner) = hir.types_module[expression.ty].clone() else {
                    unreachable!("Type of null should be a nullable");
                };
                let inner_ty = self.get_or_create_ir_type(&inner, hir, context.ir())?;
                context.emit(Opcode::Zeroed, smallvec![], inner_ty)
            }
            HirExpressionKind::ArrayIndex(arr, index) => {
                let index = self.lower_expression(*index, hir, context)?;
                let arr = self.lower_expression(*arr, hir, context)?;
                let ty = self.get_or_create_ir_type(&expression.ty, hir, context.ir())?;
                context.emit(Opcode::ArrayGet, smallvec![arr, index], ty)
            }
            HirExpressionKind::Array(arr) => {
                let values = arr
                    .iter()
                    .map(|expr| self.lower_expression(*expr, hir, context))
                    .collect::<Result<Vec<_>, _>>()?;
                let value_type = self.get_or_create_ir_type(&expression.ty, hir, context.ir())?;
                context.emit(Opcode::Array, values, value_type)
            }
            HirExpressionKind::Vector(vec) => {
                let values = vec
                    .iter()
                    .map(|expr| self.lower_expression(*expr, hir, context))
                    .collect::<Result<Vec<_>, _>>()?;
                let value_type = self.get_or_create_ir_type(&expression.ty, hir, context.ir())?;
                context.emit(Opcode::Vector, values, value_type)
            }
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
            HirExpressionKind::FunctionCall { name, args, .. } => {
                self.lower_function_call(*name, args, hir, context)?
            }
            HirExpressionKind::Binary { lhs, op, rhs } => {
                self.handle_binary_expression(*lhs, *rhs, op, hir, context)?
            }
            HirExpressionKind::Identifier(id) => {
                if let Some(value) = context.get_variable(*id) {
                    match self.ownership.expression_use(expr.data) {
                        Some(ExpressionUse::Move) => context.mov(value),
                        Some(ExpressionUse::Borrow) | Some(ExpressionUse::BorrowMut) => {
                            // Reference-taking is handled by the Reference expression kind
                            value
                        }
                        _ => value,
                    }
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
            HirExpressionKind::Enum { variant, args, .. } => {
                self.lower_enum(context, hir, expression.ty, *variant, args)?
            }
            HirExpressionKind::Matches {
                value,
                variant,
                args,
            } => self.lower_matches(value, *variant, args, hir, context)?,
        };
        if let HirType::Nullable(_) = &hir.types_module[expression.ty] {
            let bool_ty = context.ir().bool_type();
            let bool_value = context.emit_const(
                Operand::Bool(matches!(expression.kind, HirExpressionKind::Null)),
                bool_ty,
            );
            let nullable_type = self.get_or_create_ir_type(&expression.ty, hir, context.ir())?; //since its nullable, its certain for it to be an struct at this moment, so we can emit it like so
            Ok(context.emit(Opcode::Struct, smallvec![value, bool_value], nullable_type))
        } else {
            Ok(value)
        }
    }

    fn lower_matches(
        &mut self,
        hir_value: &Spanned<PoolId<HirExpression>>,
        variant: usize,
        args: &[Spanned<PoolId<HirExpression>>],
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<Value, CodegenError> {
        let value = self.lower_expression(*hir_value, hir, ctx)?;

        let then_label = ctx.create_label("matches_then");
        let end_label = ctx.create_label("matches_end");
        let int_type = ctx.ir().int_type();
        let bool_type = ctx.ir().bool_type();
        let false_value = ctx.emit_const(Operand::Bool(false), bool_type);
        let expr_view = hir.view(hir_value.data);
        let enum_type = expr_view.ty_viewer().dereference();
        let layout = self
            .enum_layouts
            .get(&enum_type.data())
            .ok_or_else(|| {
                CodegenError::InternalError(
                    "enum layout for matches target is not registered".into(),
                )
            })?
            .clone();
        let discriminant = enum_type
            .is_enum()
            .expect("Expected type of value on matches expression to be an enum")
            .variants()[variant]
            .discriminant;

        let cond = {
            let tag_value = ctx.get_field(value, 0);
            let variant_int = ctx.emit_const(Operand::Int(discriminant as i64), int_type);
            ctx.cmp(tag_value, variant_int)
        };

        if args.is_empty() {
            return Ok(cond);
        }
        // A non-empty pattern implies the matched variant carries a payload, so
        // the enum must have the payload union registered in its layout.
        layout.union_type.ok_or_else(|| {
            CodegenError::InternalError(
                "matched variant carries a payload but the enum has no payload union".into(),
            )
        })?;
        ctx.branch_conditional(cond, then_label, end_label, &[], &[false_value]);
        let union_value = ctx.get_field(value, 1);

        {
            //then label
            //pretty simple idea. this is the same as tag && payload.field0 == arg0 && payload.field1 == arg1 && ...
            //this can be implemented as
            //main:
            // cmpbranch tag == payload.field0, then, end(false)
            // then:
            //  cmpbranch payload.field1 == arg1, then2, end(false)
            //then2:
            //  cmpbranch payload.field2 == arg2, then3, end(false)
            //then3: in case, the last one, whatever
            //  br else(payload.field3 == arg3)
            //
            //This might be able to optimize by
            //then2:
            // cmp branch payload.field2 == arg2, end(payload.field3 == arg3), end(false).
            //But this will not be a thing yet
            ctx.switch_to_block(then_label).unwrap();
            let mut args = args
                .into_iter()
                .map(|arg| self.lower_expression(*arg, hir, ctx))
                .collect::<Result<Vec<_>, _>>()?;
            let (fields, last_field, last_arg): (Vec<_>, _, _) = {
                let payload = ctx.get_field(union_value, variant as u16);
                let mut fields: Vec<Value> = args
                    .iter()
                    .enumerate()
                    .map(|(i, _)| ctx.get_field(payload, i as u16))
                    .collect();
                let last_field = fields
                    .pop()
                    .expect("previous check found fields not empty but it is?");
                let last_arg = args
                    .pop()
                    .expect("previous check found args not empty but it is?");
                (fields, last_field, last_arg)
            };
            let mut current_label: IRPointer<Label, 1>;
            for (field, arg) in fields.into_iter().zip(args) {
                let field_check = ctx.cmp(arg, field);
                let then_next = ctx.create_label("then_next_field");
                ctx.branch_conditional(field_check, then_next, end_label, &[], &[false_value]);
                current_label = then_next;
                ctx.switch_to_block(current_label).unwrap();
            }
            let last_cmp = ctx.cmp(last_arg, last_field);
            ctx.branch(end_label, &[last_cmp]);
        };

        Ok(ctx.block_param(end_label, 0))
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
