use std::mem::discriminant;

use crate::hir::{
    ExpressionId, Result, SlynxHir, TypeId,
    error::{HIRError, HIRErrorKind},
    model::HirType,
    model::{FieldMethod, HirExpression, HirExpressionKind, HirStatementKind},
};
use common::{
    SymbolPointer,
    ast::{ASTExpression, ASTExpressionKind, NamedExpr, Operator, Span},
};

impl SlynxHir {
    pub fn organized_object_fields(
        &mut self,
        ty: TypeId,
        fields: Vec<NamedExpr>,
        span: &Span,
    ) -> Result<HirExpressionKind> {
        let Some(defined_layout) = self.retrieve_object_fields(ty) else {
            unreachable!(
                "The deffinition of this should have been defined during hoisting and the resolving of it"
            )
        };
        let defined_layout = defined_layout.to_vec();
        match defined_layout.len() != fields.len() {
            true if defined_layout.len() > fields.len() => {
                let missing_fields = defined_layout
                    .iter()
                    .filter_map(|field| {
                        fields
                            .iter()
                            .any(|f| self.modules.intern_name(&f.name) == *field)
                            .then_some(*field)
                    })
                    .collect::<Vec<_>>();
                return Err(HIRError::missing_properties(missing_fields, *span));
            }
            true if defined_layout.len() < fields.len() => {
                let non_existent_fields = fields
                    .iter()
                    .filter_map(|provided_field| {
                        let field_symbol = self.modules.intern_name(&provided_field.name);
                        (!defined_layout.contains(&field_symbol)).then_some(field_symbol)
                    })
                    .collect();
                return Err(HIRError::property_unrecognized(non_existent_fields, *span));
            }
            _ => {}
        }
        let mut out = Vec::with_capacity(fields.len());
        let mut non_recognized_fields = Vec::with_capacity(fields.len());

        let HirType::Struct {
            fields: field_types,
        } = self.get_type_from_ref(ty).clone()
        else {
            unreachable!();
        };
        for field in fields {
            if let Some(field_idx) = defined_layout
                .iter()
                .position(|defined_field| &self.modules.intern_name(&field.name) == defined_field)
            {
                let ty = field_types[field_idx];
                out.insert(
                    field_idx.max(out.len()),
                    self.resolve_expr(field.expr, Some(ty))?,
                );
            } else {
                let field_symbol = self.modules.intern_name(&field.name);
                non_recognized_fields.push(field_symbol);
            }
        }
        if non_recognized_fields.is_empty() {
            Ok(HirExpressionKind::Object {
                name: ty,
                fields: out,
            })
        } else {
            Err(HIRError::property_unrecognized(
                non_recognized_fields,
                *span,
            ))
        }
    }

    fn resolve_tuple_access_type(&self, ty: TypeId, index: usize, span: &Span) -> Result<TypeId> {
        // Follow the shape of the parent expression until we reach the concrete
        // tuple type that owns the requested index.
        let current_ty = self.get_type(&ty).clone();
        match current_ty {
            HirType::VarReference(variable_id) => {
                let variable_ty = self
                    .get_variable_type(variable_id)
                    .expect("variable type should exist before tuple access lowering");

                self.resolve_tuple_access_type(*variable_ty, index, span)
            }
            HirType::Field(field_method) => {
                let field_ty = self.resolve_field_method_type(&field_method, span)?;
                self.resolve_tuple_access_type(field_ty, index, span)
            }
            HirType::Reference { rf, .. } => self.resolve_tuple_access_type(rf, index, span),
            HirType::Tuple { fields } => fields.get(index).copied().ok_or(HIRError {
                kind: HIRErrorKind::InvalidTupleIndex {
                    index,
                    length: fields.len(),
                },
                span: *span,
            }),
            other => Err(HIRError {
                kind: HIRErrorKind::InvalidTupleAccessTarget { ty: other },
                span: *span,
            }),
        }
    }

    fn resolve_field_method_type(&self, field_method: &FieldMethod, span: &Span) -> Result<TypeId> {
        match field_method {
            FieldMethod::Type(rf, index) => {
                let object_ref = self.resolve_object_reference_type(*rf, span)?;
                let HirType::Struct { fields } = self.get_type_from_ref(object_ref).clone() else {
                    unreachable!("object layouts should always resolve to structs");
                };
                Ok(fields[*index])
            }
            FieldMethod::Variable(variable_id, field_name) => {
                let variable_ty = *self
                    .get_variable_type(*variable_id)
                    .expect("variable type should exist before field access lowering");
                let object_ref = self.resolve_object_reference_type(variable_ty, span)?;
                let Some(layout) = self.retrieve_object_fields(object_ref) else {
                    unreachable!("object reference should carry a layout");
                };
                let HirType::Struct { fields } = self.get_type_from_ref(object_ref).clone() else {
                    unreachable!("object layouts should always resolve to structs");
                };
                let Some(index) = layout.iter().position(|field| field == field_name) else {
                    return Err(HIRError::property_unrecognized(vec![*field_name], *span));
                };
                Ok(fields[index])
            }
            FieldMethod::Tuple(rf, index) => self.resolve_tuple_access_type(*rf, *index, span),
        }
    }

    fn resolve_object_reference_type(&self, ty: TypeId, span: &Span) -> Result<TypeId> {
        // Chained accesses can arrive here through variables, aliases, or
        // previous field accesses, so normalize them into the object reference
        // that actually owns the named layout.
        let current_ty = self.get_type(&ty).clone();
        match current_ty {
            HirType::VarReference(variable_id) => {
                let variable_ty = *self
                    .get_variable_type(variable_id)
                    .expect("variable type should exist before field access lowering");
                self.resolve_object_reference_type(variable_ty, span)
            }
            HirType::Field(field_method) => {
                let field_ty = self.resolve_field_method_type(&field_method, span)?;
                self.resolve_object_reference_type(field_ty, span)
            }
            HirType::Reference { rf, .. } => {
                if self.retrieve_object_fields(ty).is_some() {
                    Ok(ty)
                } else {
                    self.resolve_object_reference_type(rf, span)
                }
            }
            other => Err(HIRError {
                kind: HIRErrorKind::InvalidFieldAccessTarget { ty: other },
                span: *span,
            }),
        }
    }

    /// Resolves the provided `expr` trying to infer its type, if not able, keeps as infer, and on later phases fallsback to the default value.
    /// Ty only serves to tell the type of the expression if it's needed to infer and check if it doesnt correspond
    pub fn resolve_expr(
        &mut self,
        expr: ASTExpression,
        ty: Option<TypeId>,
    ) -> Result<HirExpression> {
        match expr.kind {
            ASTExpressionKind::Tuple(vector) => {
                let mut types = Vec::new();
                let mut hir_elements = Vec::new();
                for element in vector {
                    let resolved = self.resolve_expr(element, None)?;
                    types.push(resolved.ty);
                    hir_elements.push(resolved);
                }
                let tuple_ty = self.add_unnamed_type(HirType::new_tuple(types));
                Ok(self.create_tuple_expression(tuple_ty, hir_elements, expr.span))
            }
            ASTExpressionKind::TupleAccess { tuple, index } => {
                let tuple = self.resolve_expr(*tuple, None)?;
                let tuple_field_ty = self.add_unnamed_type(HirType::Field(
                    // Keep tuple accesses distinct from object fields so later
                    // phases can reject numeric access on non-tuples.
                    FieldMethod::Tuple(tuple.ty, index),
                ));

                Ok(self.create_field_access_expression(tuple, index, tuple_field_ty, expr.span))
            }

            ASTExpressionKind::If {
                condition,
                body,
                else_body,
            } => {
                let condition = self.resolve_expr(*condition, Some(self.bool_type()))?;
                let then_block: Vec<_> = body
                    .into_iter()
                    .map(|stmt| self.resolve_statement(stmt))
                    .collect::<Result<_>>()?;
                let then_type = match then_block.last().map(|s| &s.kind) {
                    Some(HirStatementKind::Expression { expr }) => expr.ty,
                    Some(HirStatementKind::Variable { value, .. }) => value.ty,
                    _ => self.void_type(),
                };
                let else_block: Option<Vec<_>> = if let Some(else_body) = else_body {
                    Some(
                        else_body
                            .into_iter()
                            .map(|stmt| self.resolve_statement(stmt))
                            .collect::<Result<_>>()?,
                    )
                } else {
                    None
                };
                let else_type = match then_block.last().map(|s| &s.kind) {
                    Some(HirStatementKind::Expression { expr }) => expr.ty,
                    Some(HirStatementKind::Variable { value, .. }) => value.ty,
                    _ => self.void_type(),
                };
                let final_type = if then_type == self.infer_type() {
                    else_type
                } else {
                    then_type
                };

                Ok(HirExpression {
                    id: ExpressionId::new(),
                    ty: final_type,
                    kind: HirExpressionKind::If {
                        condition: Box::new(condition),
                        then_branch: then_block,
                        else_branch: else_block,
                    },
                    span: expr.span,
                })
            }

            ASTExpressionKind::FunctionCall { name, args } => {
                let func_symbol = self.modules.intern_name(&name.identifier);
                let Some((decl, tyid)) = self.modules.get_declaration_by_name(&func_symbol) else {
                    return Err(HIRError::name_unrecognized(func_symbol, expr.span));
                };
                let ty = self.get_type(&tyid);
                let HirType::Function {
                    return_type,
                    args: expect_args,
                } = ty
                else {
                    return Err(HIRError::not_a_func(func_symbol, ty.clone(), expr.span));
                };
                if expect_args.len() != args.len() {
                    return Err(HIRError::invalid_funcall_arg_length(
                        func_symbol,
                        expect_args.len(),
                        args.len(),
                        expr.span,
                    ));
                }
                let return_type = *return_type;
                let exprs = args
                    .into_iter()
                    .map(|v| self.resolve_expr(v, None))
                    .collect::<Result<Vec<_>>>()?;

                Ok(HirExpression {
                    id: ExpressionId::new(),
                    ty: return_type,
                    kind: HirExpressionKind::FunctionCall {
                        name: decl,
                        args: exprs,
                    },
                    span: expr.span,
                })
            }
            ASTExpressionKind::Boolean(b) => Ok(self.create_boolean_expression(b, expr.span)),
            ASTExpressionKind::Binary { lhs, op, rhs } => self.resolve_binary(*lhs, op, *rhs, ty),
            ASTExpressionKind::StringLiteral(s) => {
                Ok(self.create_strliteral_expression(s, expr.span))
            }
            ASTExpressionKind::Identifier(name) => {
                let name = self.modules.intern_name(&name);
                let id = self.get_variable(name, &expr.span)?;
                let tyid = self.add_type(name, HirType::VarReference(id));
                Ok(self.create_identifier_expression(id, tyid, expr.span))
            }
            ASTExpressionKind::IntLiteral(int) => Ok(self.create_int_expression(int, expr.span)),
            ASTExpressionKind::FloatLiteral(float) => {
                Ok(self.create_float_expression(float, expr.span))
            }
            ASTExpressionKind::Component(component) => {
                let (id, _) =
                    self.retrieve_information_of_type(&component.name.identifier, &component.span)?;

                Ok(HirExpression {
                    kind: HirExpressionKind::Component {
                        name: id,
                        values: self.resolve_component_members(component.values, id)?,
                    },
                    id: ExpressionId::new(), // Changed to ExpressionId
                    ty: id,
                    span: expr.span,
                })
            }
            ASTExpressionKind::ObjectExpression { name, fields } => {
                let symbol = self.modules.intern_name(&name.identifier);
                let Some((_, ty)) = self.modules.get_declaration_by_name(&symbol) else {
                    return Err(HIRError::name_unrecognized(symbol, name.span));
                };
                let kind = self.organized_object_fields(ty, fields, &expr.span)?;
                Ok(HirExpression {
                    id: ExpressionId::new(),
                    ty,
                    kind,
                    span: expr.span,
                })
            }
            ASTExpressionKind::FieldAccess { parent, field } => {
                let field_symbol = self.modules.intern_name(&field);
                let parent = self.resolve_expr(*parent, None)?;
                let HirExpression { ref ty, .. } = parent;
                match self.get_type(ty) {
                    HirType::Reference { rf, .. }
                        if let Some(decl) = self.retrieve_object_fields(*rf)
                            && let Some(index) = Self::find_name_index(decl, field_symbol) =>
                    {
                        let ty = self.add_unnamed_type(HirType::type_field(*ty, index));
                        Ok(self.create_field_access_expression(parent, index, ty, expr.span))
                    }
                    HirType::Reference { .. } => Err(HIRError::property_unrecognized(
                        vec![field_symbol],
                        expr.span,
                    )),

                    HirType::VarReference(rf) => {
                        let ty = self.add_unnamed_type(HirType::variable_field(*rf, field_symbol));
                        Ok(self.create_field_access_expression(parent, usize::MAX, ty, expr.span))
                    }
                    HirType::Field(_) => {
                        let object_ref = self.resolve_object_reference_type(*ty, &expr.span)?;
                        let field = self.modules.intern_name(&field);
                        let Some(layout) = self.retrieve_object_fields(object_ref) else {
                            unreachable!("object reference should carry a layout");
                        };
                        match Self::find_name_index(layout, field) {
                            Some(index) => {
                                let field_ty =
                                    self.add_unnamed_type(HirType::type_field(object_ref, index));
                                Ok(self.create_field_access_expression(
                                    parent, index, field_ty, expr.span,
                                ))
                            }
                            _ => Err(HIRError::property_unrecognized(vec![field], expr.span)),
                        }
                    }
                    u => Err(HIRError {
                        kind: HIRErrorKind::InvalidFieldAccessTarget { ty: u.clone() },
                        span: expr.span,
                    }),
                }
            }
        }
    }

    fn find_name_index(names: &[SymbolPointer], target: SymbolPointer) -> Option<usize> {
        names
            .iter()
            .position(|struct_field| *struct_field == target)
    }

    ///Resolves the binary operation with the provided `lhs` and `rhs`.
    pub fn resolve_binary(
        &mut self,
        lhs: ASTExpression,
        op: Operator,
        rhs: ASTExpression,
        ty: Option<TypeId>,
    ) -> Result<HirExpression> {
        let mut lhs = self.resolve_expr(lhs, ty)?;
        let mut rhs = self.resolve_expr(rhs, ty)?;
        match discriminant(self.get_type(&lhs.ty)) == discriminant(self.get_type(&rhs.ty)) {
            false if lhs.ty == self.infer_type() => lhs.ty = rhs.ty,
            false if rhs.ty == self.infer_type() => rhs.ty = lhs.ty,
            _ => {}
        }
        let span = lhs.span.merge_with(rhs.span);
        let expr_type = match op {
            Operator::Add
            | Operator::Sub
            | Operator::Star
            | Operator::Slash
            | Operator::RightShift
            | Operator::LeftShift
            | Operator::Xor
            | Operator::And
            | Operator::Or => lhs.ty,
            Operator::Equals
            | Operator::LogicAnd
            | Operator::LogicOr
            | Operator::GreaterThan
            | Operator::GreaterThanOrEqual
            | Operator::LessThan
            | Operator::LessThanOrEqual => self.bool_type(),
        };
        Ok(self.create_binary_expression(lhs, rhs, op, expr_type, span))
    }
}
