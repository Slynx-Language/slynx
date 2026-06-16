use std::mem::discriminant;

use crate::{
    ExpressionId, Result, SlynxHir, SymbolPointer, TypeId,
    error::{HIRError, HIRErrorKind},
    model::{FieldMethod, HirExpression, HirExpressionKind, HirStatementKind, HirType},
    module_loader::FileId,
};
use common::{Operator, Span};
use slynx_parser::{ASTExpression, ASTExpressionKind, ASTStatement, GenericIdentifier, NamedExpr};

impl SlynxHir {
    ///Generates a new object expression for the given `ty` object, and the given `fields`.
    pub(crate) fn generate_object_expression(
        &self,
        file: FileId,
        ty: TypeId,
        fields: &[NamedExpr],
        span: Span,
    ) -> Result<HirExpression> {
        let Some(defined_layout) = self.get_object_fields(ty, file) else {
            unreachable!(
                "The definition of this should have been defined during hoisting and the resolving of it"
            )
        };
        let defined_layout = defined_layout.to_vec();
        match true {
            _ if defined_layout.len() > fields.len() => {
                let missing_fields = defined_layout
                    .iter()
                    .filter_map(|field| {
                        fields
                            .iter()
                            .any(|f| self.intern_name(&f.name) == *field)
                            .then_some(*field)
                    })
                    .collect::<Vec<_>>();
                return Err(HIRError::missing_properties(missing_fields, span));
            }
            _ if defined_layout.len() < fields.len() => {
                let non_existent_fields = fields
                    .iter()
                    .filter_map(|provided_field| {
                        let field_symbol = self.intern_name(&provided_field.name);
                        (!defined_layout.contains(&field_symbol)).then_some(field_symbol)
                    })
                    .collect();
                return Err(HIRError::property_unrecognized(
                    ty,
                    non_existent_fields,
                    span,
                ));
            }
            _ => {}
        }
        let mut resultant_fields = Vec::with_capacity(fields.len());
        let mut non_recognized_fields = Vec::with_capacity(fields.len());
        let HirType::Struct {
            fields: field_types,
        } = self.get_type_from_ref(ty, &span)?.clone()
        else {
            unreachable!();
        };
        for field in fields {
            if let Some(field_idx) = defined_layout
                .iter()
                .position(|defined_field| &self.intern_name(&field.name) == defined_field)
            {
                let ty = field_types[field_idx];
                resultant_fields.insert(
                    field_idx.max(resultant_fields.len()),
                    self.generate_expression(file, &field.expr, Some(ty))?,
                );
            } else {
                let field_symbol = self.intern_name(&field.name);
                non_recognized_fields.push(field_symbol);
            }
        }
        if non_recognized_fields.is_empty() {
            Ok(HirExpression {
                id: ExpressionId::new(),
                ty,
                kind: HirExpressionKind::Object {
                    name: ty,
                    fields: resultant_fields,
                },
                span,
            })
        } else {
            Err(HIRError::property_unrecognized(
                ty,
                non_recognized_fields,
                span,
            ))
        }
    }

    fn resolve_tuple_access_type(
        &self,
        file: FileId,
        ty: TypeId,
        index: usize,
        span: &Span,
    ) -> Result<TypeId> {
        // Follow the shape of the parent expression until we reach the concrete
        // tuple type that owns the requested index.
        let current_ty = self.get_type(&ty).clone();
        match current_ty {
            HirType::VarReference(variable_id) => {
                let variable_ty = self
                    .get_variable_type(variable_id)
                    .expect("variable type should exist before tuple access lowering");

                self.resolve_tuple_access_type(file, variable_ty, index, span)
            }
            HirType::Field(field_method) => {
                let field_ty = self.resolve_field_method_type(file, &field_method, span)?;
                self.resolve_tuple_access_type(file, field_ty, index, span)
            }
            HirType::Reference { rf, .. } => self.resolve_tuple_access_type(file, rf, index, span),
            HirType::Tuple { fields } => fields
                .get(index)
                .copied()
                .ok_or(HIRError::invalid_tuple_index(index, fields.len(), *span)),
            other => Err(HIRError::invalid_tuple_target(other, *span)),
        }
    }

    fn resolve_field_method_type(
        &self,
        file: FileId,
        field_method: &FieldMethod,
        span: &Span,
    ) -> Result<TypeId> {
        match field_method {
            FieldMethod::Type(rf, index) => {
                let object_ref = self.resolve_object_reference_type(file, *rf, span)?;
                let HirType::Struct { fields } = self.get_type_from_ref(object_ref, span)?.clone()
                else {
                    unreachable!("object layouts should always resolve to structs");
                };
                Ok(fields[*index])
            }
            FieldMethod::Variable(variable_id, field_name) => {
                let variable_ty = self
                    .get_variable_type(*variable_id)
                    .expect("variable type should exist before field access lowering");
                let object_ref = self.resolve_object_reference_type(file, variable_ty, span)?;
                let reader = self.get_type_from_ref(object_ref, span)?;
                let (layout, fields) = match (self.get_object_fields(object_ref, file), &*reader) {
                    (Some(layout), HirType::Struct { fields }) => (layout, fields),
                    (None, _) => unreachable!("object reference should carry a layout"),
                    (_, _) => unreachable!("object layouts should always resolve to structs"),
                };

                match Self::find_name_index(&layout, *field_name) {
                    Some(index) => Ok(fields[index]),
                    None => Err(HIRError::property_unrecognized(
                        object_ref,
                        vec![*field_name],
                        *span,
                    )),
                }
            }
            FieldMethod::Tuple(rf, index) => {
                self.resolve_tuple_access_type(file, *rf, *index, span)
            }
        }
    }

    fn resolve_object_reference_type(
        &self,
        file: FileId,
        ty: TypeId,
        span: &Span,
    ) -> Result<TypeId> {
        // Chained accesses can arrive here through variables, aliases, or
        // previous field accesses, so normalize them into the object reference
        // that actually owns the named layout.
        let current_ty = self.get_type(&ty).clone();
        match current_ty {
            HirType::VarReference(variable_id) => {
                let variable_ty = self
                    .get_variable_type(variable_id)
                    .expect("variable type should exist before field access lowering");
                self.resolve_object_reference_type(file, variable_ty, span)
            }
            HirType::Field(field_method) => {
                let field_ty = self.resolve_field_method_type(file, &field_method, span)?;
                self.resolve_object_reference_type(file, field_ty, span)
            }
            HirType::Reference { rf, .. } => {
                if self.get_object_fields(ty, file).is_some() {
                    Ok(ty)
                } else {
                    self.resolve_object_reference_type(file, rf, span)
                }
            }
            other => Err(HIRError {
                kind: HIRErrorKind::InvalidFieldAccessTarget { ty: other },
                span: *span,
            }),
        }
    }

    /// Resolves an `if` expression, type-checking the condition and both branches.
    pub(crate) fn resolve_if_expression(
        &self,
        file: FileId,
        condition: &ASTExpression,
        if_body: &[ASTStatement],
        else_body: Option<&[ASTStatement]>,
        span: Span,
    ) -> Result<HirExpression> {
        let condition = self.generate_expression(file, condition, Some(self.bool_type()))?;
        let then_block: Vec<_> = if_body
            .iter()
            .map(|stmt| self.resolve_statement(file, stmt))
            .collect::<Result<_>>()?;
        let then_type = match then_block.last().map(|s| &s.kind) {
            Some(HirStatementKind::Expression { expr }) => expr.ty,
            Some(HirStatementKind::Variable { value, .. }) => value.ty,
            _ => self.void_type(),
        };
        let else_block: Option<Vec<_>> = else_body
            .map(|body| {
                body.iter()
                    .map(|stmt| self.resolve_statement(file, stmt))
                    .collect::<Result<_>>()
            })
            .transpose()?;
        let else_type = match else_block.as_ref().and_then(|b| b.last()).map(|s| &s.kind) {
            Some(HirStatementKind::Expression { expr }) => expr.ty,
            Some(HirStatementKind::Variable { value, .. }) => value.ty,
            _ => self.void_type(),
        };
        let final_type = if then_type == self.infer_type() {
            else_type
        } else {
            then_type
        };
        Ok(self.create_if_expression(condition, then_block, else_block, final_type, span))
    }

    ///Generates a new tuple expression
    fn generate_tuple(
        &self,
        file: FileId,
        values: &[ASTExpression],
        span: Span,
    ) -> Result<HirExpression> {
        let mut types = Vec::new();
        let mut hir_elements = Vec::new();
        for element in values {
            let resolved = self.generate_expression(file, element, None)?;
            types.push(resolved.ty);
            hir_elements.push(resolved);
        }
        let tuple_ty = self.create_unnamed_type(HirType::new_tuple(types));
        Ok(self.create_tuple_expression(tuple_ty, hir_elements, span))
    }
    ///Generates a new tuple expression
    fn generate_tuple_access(
        &self,
        file: FileId,
        tuple: &ASTExpression,
        index: usize,
        span: Span,
    ) -> Result<HirExpression> {
        let tuple = self.generate_expression(file, tuple, None)?;
        let tuple_field_ty =
            self.create_unnamed_type(HirType::Field(FieldMethod::Tuple(tuple.ty, index)));
        Ok(self.create_field_access_expression(tuple, index, None, tuple_field_ty, span))
    }

    fn generate_funcall(
        &self,
        file: FileId,
        name: &GenericIdentifier,
        args: &[ASTExpression],
        span: Span,
    ) -> Result<HirExpression> {
        let func_symbol = self.intern_name(&name.identifier);
        let (decl, tyid) = self.find_declaration_by_name(&func_symbol, span)?;
        let (return_type, expect_args) = {
            let ty = self.get_type(&tyid);
            let HirType::Function {
                return_type,
                args: expect_args,
            } = &*ty
            else {
                return Err(HIRError::not_a_func(func_symbol, ty.clone(), span));
            };
            (*return_type, expect_args.clone())
        };
        if expect_args.len() != args.len() {
            return Err(HIRError::invalid_funcall_arg_length(
                func_symbol,
                expect_args.len(),
                args.len(),
                span,
            ));
        }
        let exprs = match args
            .iter()
            .map(|v| self.generate_expression(file, v, None))
            .collect::<Result<Vec<_>>>()
        {
            Ok(exprs) => exprs,
            Err(mut e) => {
                e.span = span;
                return Err(e);
            }
        };

        Ok(HirExpression {
            id: ExpressionId::new(),
            ty: return_type,
            kind: HirExpressionKind::FunctionCall {
                name: decl,
                args: exprs,
            },
            span,
        })
    }

    fn generate_identifier_field(
        &self,
        file: FileId,
        parent: HirExpression,
        field_symbol: SymbolPointer,
        span: Span,
    ) -> Result<HirExpression> {
        let HirExpression { ref ty, .. } = parent;
        let ty_kind = self.get_type(ty);
        match &*ty_kind {
            HirType::Reference { rf, .. }
                if let Some(decl) = self.get_object_fields(rf.clone(), file)
                    && let Some(index) = Self::find_name_index(&decl, field_symbol) =>
            {
                let ty = self.create_unnamed_type(HirType::type_field(*ty, index));
                Ok(
                    self.create_field_access_expression(
                        parent,
                        index,
                        Some(field_symbol),
                        ty,
                        span,
                    ),
                )
            }
            HirType::Reference { .. } => Err(HIRError::property_unrecognized(
                ty.clone(),
                vec![field_symbol],
                span,
            )),

            HirType::VarReference(rf) => {
                let ty =
                    self.create_unnamed_type(HirType::variable_field(rf.clone(), field_symbol));
                Ok(self.create_field_access_expression(
                    parent,
                    usize::MAX,
                    Some(field_symbol),
                    ty,
                    span,
                ))
            }
            HirType::Field(_) => {
                let object_ref = self.resolve_object_reference_type(file, *ty, &span)?;
                let Some(layout) = self.get_object_fields(object_ref, file) else {
                    unreachable!("object reference should carry a layout");
                };
                match Self::find_name_index(&layout, field_symbol) {
                    Some(index) => {
                        let field_ty =
                            self.create_unnamed_type(HirType::type_field(object_ref, index));
                        Ok(self.create_field_access_expression(
                            parent,
                            index,
                            Some(field_symbol),
                            field_ty,
                            span,
                        ))
                    }
                    _ => Err(HIRError::property_unrecognized(
                        object_ref,
                        vec![field_symbol],
                        span,
                    )),
                }
            }
            u => Err(HIRError {
                kind: HIRErrorKind::InvalidFieldAccessTarget { ty: u.clone() },
                span,
            }),
        }
    }
    fn generate_field_access_on(
        &self,
        file: FileId,
        parent_hir: HirExpression,
        field_ast: &ASTExpression,
        span: Span,
    ) -> Result<HirExpression> {
        match &field_ast.kind {
            ASTExpressionKind::Identifier(name) => {
                let symbol = self.intern_name(name);
                self.generate_identifier_field(file, parent_hir, symbol, span)
            }

            ASTExpressionKind::FieldAccess {
                parent: inner_parent,
                field: inner_field,
            } => {
                // Recursão para o próximo nível
                let intermediate = self.generate_field_access_on(
                    file,
                    parent_hir,     // parent atual vira o novo parent
                    inner_parent,   // o "parent" do field aninhado
                    field_ast.span, // ou inner_parent.span
                )?;

                self.generate_field_access_on(file, intermediate, inner_field, span)
            }

            _ => panic!(
                "This function should be called only when `field ast` is an identifier of field access"
            ),
        }
    }
    fn generate_field_access_expression(
        &self,
        file: FileId,
        parent: &ASTExpression,
        field: &ASTExpression,
        span: Span,
    ) -> Result<HirExpression> {
        let parent_expr = self.generate_expression(file, parent, None);

        match &field.kind {
            ASTExpressionKind::Identifier(_) | ASTExpressionKind::FieldAccess { .. } => {
                self.generate_field_access_on(file, parent_expr?, field, field.span)
            }
            ASTExpressionKind::FunctionCall { name, args } => {
                let name = self.intern_name(&name.identifier);
                match parent_expr {
                    Ok(parent_expr) => {
                        let args = args
                            .iter()
                            .map(|arg| self.generate_expression(file, arg, None))
                            .collect::<Result<_>>()?;
                        Ok(self.create_method_call_expression(parent_expr, name, args, span))
                    }
                    Err(err) if matches!(err.kind, HIRErrorKind::NameNotRecognized(_)) => {
                        if let ASTExpressionKind::Identifier(ident) = &parent.kind {
                            let type_symbol = self.intern_name(ident);
                            if let Ok(type_ty) = self.get_type_of_name(type_symbol, &parent.span) {
                                if let Some(method_decl) = self
                                    .types_module
                                    .methods
                                    .get(&type_ty)
                                    .and_then(|m| m.get(&name).map(|entry| *entry.value()))
                                {
                                    let args = args
                                        .iter()
                                        .map(|arg| self.generate_expression(file, arg, None))
                                        .collect::<Result<_>>()?;
                                    return Ok(HirExpression {
                                        id: ExpressionId::new(),
                                        ty: self.infer_type(),
                                        kind: HirExpressionKind::FunctionCall {
                                            name: method_decl,
                                            args,
                                        },
                                        span,
                                    });
                                }
                                return Err(HIRError::invalid_field_access(span));
                            }
                        }
                        Err(err)
                    }
                    Err(err) => Err(err),
                }
            }

            _ => Err(HIRError::invalid_field_access(field.span)),
        }
    }

    /// Resolves the provided `expr` trying to infer its type, if not able, keeps as infer, and on later phases fallsback to the default value.
    /// Ty only serves to tell the type of the expression if it's needed to infer and check if it doesnt correspond
    pub(crate) fn generate_expression(
        &self,
        file: FileId,
        expr: &ASTExpression,
        ty: Option<TypeId>,
    ) -> Result<HirExpression> {
        match &expr.kind {
            ASTExpressionKind::Tuple(vector) => self.generate_tuple(file, vector, expr.span),
            ASTExpressionKind::TupleAccess { tuple, index } => {
                self.generate_tuple_access(file, tuple, *index, expr.span)
            }
            ASTExpressionKind::If {
                condition,
                body,
                else_body,
            } => self.resolve_if_expression(
                file,
                condition,
                body,
                else_body.as_ref().map(|v| &**v),
                expr.span,
            ),

            ASTExpressionKind::FunctionCall { name, args } => {
                self.generate_funcall(file, name, args, expr.span)
            }
            ASTExpressionKind::Boolean(b) => Ok(self.create_boolean_expression(*b, expr.span)),
            ASTExpressionKind::Binary { lhs, op, rhs } => {
                self.resolve_binary(file, lhs, *op, rhs, ty)
            }
            ASTExpressionKind::StringLiteral(s) => {
                let ptr = self.intern_name(s);
                Ok(self.create_strliteral_expression(ptr, expr.span))
            }
            ASTExpressionKind::Identifier(name) => {
                let name = self.intern_name(name);
                let id = self.get_variable(file, name, &expr.span)?;
                let tyid = self.create_type(name, HirType::VarReference(id));
                Ok(self.create_identifier_expression(id, tyid, expr.span))
            }
            ASTExpressionKind::IntLiteral(int) => Ok(self.create_int_expression(*int, expr.span)),
            ASTExpressionKind::FloatLiteral(float) => {
                Ok(self.create_float_expression(*float, expr.span))
            }
            ASTExpressionKind::Component(component) => {
                let symbol = self.intern_name(&component.name.identifier);
                let id = self.get_type_of_name(symbol, &component.span)?;
                let component = self.resolve_component_expression(file, component)?;
                Ok(self.create_component_expression(component, id, expr.span))
            }
            ASTExpressionKind::ObjectExpression { name, fields } => {
                let symbol = self.intern_name(&name.identifier);
                let (_, ty) = if name.identifier == "Self" {
                    let file_guard = self.get_file(file);
                    file_guard
                        .declarations
                        .get_import_alias(&symbol)
                        .ok_or_else(|| HIRError::name_unrecognized(symbol, name.span))?
                } else {
                    self.find_declaration_by_name(&symbol, name.span)?
                };
                self.generate_object_expression(file, ty, fields, expr.span)
            }
            ASTExpressionKind::FieldAccess { parent, field } => {
                self.generate_field_access_expression(file, parent, field, expr.span)
            }
        }
    }

    fn find_name_index(names: &[SymbolPointer], target: SymbolPointer) -> Option<usize> {
        names
            .iter()
            .position(|struct_field| *struct_field == target)
    }

    ///Resolves the binary operation with the provided `lhs` and `rhs`.
    pub(crate) fn resolve_binary(
        &self,
        file: FileId,
        lhs: &ASTExpression,
        op: Operator,
        rhs: &ASTExpression,
        ty: Option<TypeId>,
    ) -> Result<HirExpression> {
        let mut lhs = self.generate_expression(file, lhs, ty)?;
        let mut rhs = self.generate_expression(file, rhs, ty)?;
        let lhs_disc = {
            let lhs_reader = self.get_type(&lhs.ty);
            discriminant(&*lhs_reader)
        };
        let rhs_disc = {
            let rhs_reader = self.get_type(&rhs.ty);
            discriminant(&*rhs_reader)
        };
        match lhs_disc == rhs_disc {
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
