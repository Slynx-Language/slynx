use slynx_hir::{
    DeclarationId, HirExpression, HirExpressionKind, HirStatement, HirStatementKind, SlynxHir,
    TypeId,
};
use slynx_ir::{FunctionBuilder, IRPointer, IRTypeId, Label, Operand, SlynxIR, Value};

use crate::{Codegen, CodegenError, functions::FunctionContext};

impl Codegen {
    fn lower_if_branch(
        &mut self,
        branch: &[HirStatement],
        end_label: IRPointer<Label, 1>,
    ) -> Result<Option<IRTypeId>, CodegenError> {
        for (idx, statement) in branch.iter().enumerate() {
            if idx == branch.len() - 1
                && let HirStatementKind::Expression { ref expr } = statement.kind
            {
                let value = self.lower_expression(expr)?;
                let ty = self.get_type_of_value(value);

                // The merge label carries the value produced by each branch.
                // Its argument type is the branch result type, not the condition type.
                if self.get_label(end_label).arguments().is_empty() {
                    self.get_label_mut(end_label).add_argument(ty);
                }

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::br(end_label, value.with_length(), ty),
                    true,
                );
                return Ok(Some(ty));
            }

            if self.generate_statement(statement, temp)?.is_some() {
                return Ok(None);
            }
        }

        self.insert_instruction(
            temp.current_label(),
            Instruction::br(end_label, IRPointer::null(), self.types.void_type()),
            true,
        );
        Ok(None)
    }

    // ── helpers for generate_value_for ──────────────────────────────────

    fn lower_tuple_expression(&mut self, vetor: &[HirExpression]) -> Result<Value, IRError> {
        let values_ptrs = vetor
            .iter()
            .map(|e| self.generate_value_for(e, temp))
            .collect::<Result<Vec<_>, _>>()?;
        let mut element_types = Vec::with_capacity(values_ptrs.len());
        for ptr in &values_ptrs {
            let ty = self.get_type_of_value(*ptr);
            element_types.push(ty);
        }
        let values = values_ptrs
            .iter()
            .map(|v| self.get_value(*v))
            .collect::<Vec<_>>();
        let values_ptr = self.insert_values(&values);

        let ty = self.types.create_or_get_tuple(element_types);
        Ok(Value::new_struct(ty, values_ptr))
    }

    fn lower_string_literal(&mut self, v: SymbolPointer, temp: &mut TempIRData) -> Value {
        let operand = Operand::String(v);
        let operand_ptr = self.insert_operands(&[operand]);
        let value = self.insert_value(self.create_raw_value(operand_ptr));
        let instruction = self.insert_instruction(
            temp.current_label(),
            Instruction::raw(value, self.types.str_type()),
            false,
        );
        let ptr = self.dereference_instruction_ptr(instruction);
        Value::new_instruction(ptr.with_length(), self.types.str_type())
    }

    fn lower_literal(&mut self, expr: &HirExpression, temp: &mut TempIRData) -> Value {
        let (operand, optype) = self.generate_operand(expr).unwrap();
        let value = self.insert_value(self.create_raw_value(operand));
        let instruction =
            self.insert_instruction(temp.current_label(), Instruction::raw(value, optype), false);
        Value::new_instruction(
            self.dereference_instruction_ptr(instruction).with_length(),
            optype,
        )
    }

    fn lower_function_call(
        &mut self,
        name: DeclarationId,
        args: &[HirExpression],
    ) -> Result<Value, IRError> {
        let func = {
            let func = temp.get_function(name);
            debug_assert!(func.len() == 1);
            func.with_length::<1>()
        };
        let ret_ty = self.return_type_of_context(func);
        let mut operands = Vec::with_capacity(args.len());

        for arg in args {
            let value = self.generate_value_for(arg, temp)?;
            operands.push(value);
        }
        let ptr = self.operands.len();
        let ptr = IRPointer::new(ptr, operands.len());
        let instruction = self.insert_instruction(
            temp.current_label(),
            Instruction::call(func, ret_ty, ptr),
            false,
        );
        Ok(Value::new_instruction(
            self.dereference_instruction_ptr(instruction).with_length(),
            ret_ty,
        ))
    }

    fn lower_struct_literal(
        &mut self,
        name: TypeId,
        fields: &[HirExpression],
    ) -> Result<Value, IRError> {
        let ty = temp.get_type(name)?;
        let values = {
            let fields = fields
                .iter()
                .map(|v| self.generate_value_for(v, temp))
                .collect::<Result<Vec<_>, _>>()?;
            self.insert_values(
                &fields
                    .iter()
                    .map(|v| self.get_value(*v))
                    .collect::<Vec<_>>(),
            )
        };
        let strts = self.insert_instruction(
            temp.current_label(),
            Instruction::struct_literal(ty, values),
            false,
        );
        Ok(Value::new_instruction(
            self.dereference_instruction_ptr(strts).with_length(),
            ty,
        ))
    }

    fn lower_field_access(
        &mut self,
        expr: &HirExpression,
        field_index: usize,
    ) -> Result<Value, IRError> {
        let value = self.generate_value_for(expr, temp)?;
        let ty = self.get_type_of_value(value);
        let instruction_ptr = self.insert_instruction(
            temp.current_label(),
            Instruction::getfield(field_index, value, ty),
            false,
        );
        let instruction_ptr = self
            .dereference_instruction_ptr(instruction_ptr)
            .with_length();
        Ok(Value::new_instruction(instruction_ptr, ty))
    }

    fn lower_if_expression(
        &mut self,
        condition: &HirExpression,
        then_branch: &[HirStatement],
        else_branch: &Option<Vec<HirStatement>>,
    ) -> Result<Value, IRError> {
        let value = self.generate_value_for(condition, temp)?;
        let then_label = self.insert_label(temp.current_function(), "then_label");
        let else_label = self.insert_label(temp.current_function(), "else_label");
        let end_label = self.insert_label(temp.current_function(), "end_label");
        let condition_ty = self.get_type_of_value(value);

        self.insert_instruction(
            temp.current_label(),
            Instruction::cbr(
                value,
                then_label,
                else_label,
                IRPointer::null(),
                IRPointer::null(),
                condition_ty,
            ),
            true,
        );

        {
            let next = self.get_next_mapeable_instruction_ptr();
            let mut fresh = next.with_length::<0>();
            fresh.set_length(0);
            self.get_label_mut(then_label)
                .set_instructions_pointer(fresh);
        }

        temp.set_current_label(then_label);
        self.lower_if_branch(then_branch, end_label)?;

        {
            let next = self.get_next_mapeable_instruction_ptr();
            let mut fresh = next.with_length::<0>();
            fresh.set_length(0);
            self.get_label_mut(else_label)
                .set_instructions_pointer(fresh);
        }

        temp.set_current_label(else_label);
        let else_branch = else_branch.as_deref().unwrap_or(&[]);
        self.lower_if_branch(else_branch, end_label)?;

        {
            let next = self.get_next_mapeable_instruction_ptr();
            let mut fresh = next.with_length::<0>();
            fresh.set_length(0);
            self.get_label_mut(end_label)
                .set_instructions_pointer(fresh);
        }

        temp.set_current_label(end_label);
        let end_label = self.get_label(end_label);
        if end_label.arguments().is_empty() {
            Ok(self.create_void_value())
        } else {
            Ok(end_label.get_argument_value(0))
        }
    }

    fn generate_operand_value(op: Operand, ir: &mut SlynxIR) -> Value {
        let operand_ty = match op {
            Operand::Bool(_) => ir.bool_type(),
            Operand::Float(_) => ir.float_type(),
            Operand::Int(_) => ir.int_type(),
            Operand::String(_) => ir.str_type(),
        };
        let ptr = ir.create_single_operand(op);
        Value::new_raw(ptr, operand_ty)
    }

    /// Returns an instruction pointer for the given expression.
    pub(crate) fn lower_expression<'a>(
        &mut self,
        context: &mut FunctionContext<'a>,
        expr: &HirExpression,
        hir: &SlynxHir,
    ) -> Result<IRPointer<Value, 1>, CodegenError> {
        let ir = context.ir();
        let value = match &expr.kind {
            HirExpressionKind::Tuple(vetor) => self.lower_tuple_expression(vetor)?,
            HirExpressionKind::StringLiteral(v) => {
                let string = self.intern_to_ir(hir, ir, *v);
                Self::generate_operand_value(Operand::String(string), ir)
            }
            HirExpressionKind::Bool(v) => Self::generate_operand_value(Operand::Bool(*v), ir),
            HirExpressionKind::Float(f) => {
                Self::generate_operand_value(Operand::Float(*f as f64), ir)
            }
            HirExpressionKind::Int(i) => Self::generate_operand_value(Operand::Int(*i as i64), ir),
            HirExpressionKind::FunctionCall { name, args } => {
                self.lower_function_call(*name, args)?
            }
            HirExpressionKind::Binary { lhs, op, rhs } => {
                self.handle_binary_expression(lhs, rhs, op)?
            }
            HirExpressionKind::Identifier(id) => {
                if let Some(value) = context.get_variable(*id) {
                    value
                } else {
                    return Err(CodegenError::UnrecognizedVariable(*id));
                }
            }
            HirExpressionKind::Object { name, fields } => {
                self.lower_struct_literal(*name, fields)?
            }
            HirExpressionKind::FieldAccess { expr, field_index } => {
                self.lower_field_access(expr, *field_index)?
            }
            HirExpressionKind::Component(c) => self.get_component_expression(c)?.0,
            HirExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.lower_if_expression(condition, then_branch, else_branch)?,
        };
        Ok(ir.insert_value(value))
    }
}
