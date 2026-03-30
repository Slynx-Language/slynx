use std::mem::discriminant;

use color_eyre::eyre::Result;

use crate::hir::{
    ExpressionId, SlynxHir, TypeId,
    definitions::{HirExpression, HirExpressionKind, HirStatementKind},
    error::{HIRError, HIRErrorKind},
    types::{FieldMethod, HirType},
};
use common::ast::{ASTExpression, ASTExpressionKind, NamedExpr, Operator, Span};

impl SlynxHir {
    pub fn organized_object_fields(
        &mut self,
        ty: TypeId,
        fields: Vec<NamedExpr>,
        span: &Span,
    ) -> Result<HirExpressionKind> {
        let Some(defined_layout) = self.declarations_module.retrieve_object_body(ty) else {
            unreachable!(
                "The deffinition of this should have been defined during hoisting and the resolving of it"
            )
        };
        let defined_layout = defined_layout.to_vec();
        if defined_layout.len() != fields.len() {
            if defined_layout.len() > fields.len() {
                let missing_fields = defined_layout
                    .iter()
                    .filter_map(|field| {
                        if fields
                            .iter()
                            .any(|f| self.symbols_module.intern(&f.name) == *field)
                        {
                            None
                        } else {
                            Some(self.symbols_module[*field].to_string())
                        }
                    })
                    .collect::<Vec<_>>();
                return Err(HIRError {
                    kind: HIRErrorKind::MissingProperty {
                        prop_names: missing_fields,
                    },
                    span: span.clone(),
                }
                .into());
            } else {
                let non_existent_fields = fields
                    .into_iter()
                    .filter(|provided_field| {
                        !defined_layout
                            .iter()
                            .any(|f| *f == self.symbols_module.intern(&provided_field.name))
                    })
                    .map(|f| f.name)
                    .collect::<Vec<String>>();
                return Err(HIRError {
                    kind: HIRErrorKind::PropertyNotRecognized {
                        prop_names: non_existent_fields,
                    },
                    span: span.clone(),
                }
                .into());
            }
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
                .position(|defined_field| &self.symbols_module.intern(&field.name) == defined_field)
            {
                let ty = field_types[field_idx];
                out.insert(
                    field_idx.max(out.len()),
                    self.resolve_expr(field.expr, Some(ty))?,
                );
            } else {
                non_recognized_fields.push(field.name);
            }
        }
        if non_recognized_fields.is_empty() {
            Ok(HirExpressionKind::Object {
                name: ty,
                fields: out,
            })
        } else {
            Err(HIRError {
                kind: HIRErrorKind::PropertyNotRecognized {
                    prop_names: non_recognized_fields,
                },
                span: span.clone(),
            }
            .into())
        }
    }

    /// Ty only serves to tell the type of the expression if it's needed to infer and check if it doesnt correspond
    pub fn resolve_expr(
        &mut self,
        expr: ASTExpression,
        ty: Option<TypeId>,
    ) -> Result<HirExpression> {
        match expr.kind {
            ASTExpressionKind::If {
                condition,
                body,
                else_body,
            } => {
                let condition = self.resolve_expr(*condition, Some(self.types_module.bool_id()))?;
                let then_block: Vec<_> = body
                    .into_iter()
                    .map(|stmt| self.resolve_statement(stmt))
                    .collect::<Result<_>>()?;

                let then_type = if let Some(last) = then_block.last() {
                    match &last.kind {
                        HirStatementKind::Expression { expr } => expr.ty,
                        HirStatementKind::Variable { value, .. } => value.ty,
                        _ => self.types_module.void_id(),
                    }
                } else {
                    self.types_module.void_id()
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
                let else_type = if let Some(ref block) = else_block {
                    if let Some(last) = block.last() {
                        match &last.kind {
                            HirStatementKind::Expression { expr } => expr.ty,
                            HirStatementKind::Variable { value, .. } => value.ty,
                            _ => self.types_module.void_id(),
                        }
                    } else {
                        self.types_module.void_id()
                    }
                } else {
                    self.types_module.void_id()
                };
                let final_type = if then_type == self.types_module.infer_id() {
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
                let func_symbol = self.get_symbol_of(&name.identifier, &expr.span)?;
                let Some((decl, tyid)) = self
                    .declarations_module
                    .retrieve_declaration_data_by_name(&func_symbol)
                else {
                    return Err(HIRError {
                        kind: HIRErrorKind::NameNotRecognized(name.identifier),
                        span: expr.span,
                    }
                    .into());
                };
                let ty = self.types_module.get_type(&tyid);
                let HirType::Function {
                    return_type,
                    args: expect_args,
                } = ty
                else {
                    return Err(HIRError {
                        kind: HIRErrorKind::NotAFunction(name.identifier, ty.clone()),
                        span: expr.span,
                    }
                    .into());
                };
                if expect_args.len() != args.len() {
                    return Err(HIRError {
                        kind: HIRErrorKind::InvalidFuncallArgLength {
                            func_name: name.identifier,
                            expected_length: expect_args.len(),
                            received_length: args.len(),
                        },
                        span: expr.span,
                    }
                    .into());
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
            ASTExpressionKind::Boolean(b) => Ok(HirExpression {
                id: ExpressionId::new(),
                ty: self.types_module.bool_id(),
                kind: HirExpressionKind::Bool(b),
                span: expr.span,
            }),
            ASTExpressionKind::Binary { lhs, op, rhs } => self.resolve_binary(*lhs, op, *rhs, ty),
            ASTExpressionKind::StringLiteral(s) => Ok(HirExpression {
                id: ExpressionId::new(), // Changed to ExpressionId
                ty: self.types_module.str_id(),
                kind: HirExpressionKind::StringLiteral(s),
                span: expr.span,
            }),
            ASTExpressionKind::Identifier(name) => {
                let id = *self.get_variable(&name, &expr.span)?;
                let tyid = self
                    .types_module
                    .insert_type(self.symbols_module.intern(&name), HirType::VarReference(id));
                Ok(HirExpression {
                    kind: HirExpressionKind::Identifier(id),
                    id: ExpressionId::new(), // Changed to ExpressionId
                    ty: tyid,
                    span: expr.span,
                })
            }
            ASTExpressionKind::IntLiteral(int) => Ok(HirExpression {
                kind: HirExpressionKind::Int(int),
                ty: self.types_module.int_id(),
                id: ExpressionId::new(), // Changed to ExpressionId
                span: expr.span,
            }),

            ASTExpressionKind::FloatLiteral(float) => Ok(self.create_floatexpr(float, expr.span)),
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
                let symbol = self.symbols_module.intern(&name.identifier);
                let Some((_, ty)) = self
                    .declarations_module
                    .retrieve_declaration_data_by_name(&symbol)
                else {
                    return Err(HIRError {
                        kind: HIRErrorKind::NameNotRecognized(name.identifier),
                        span: name.span,
                    }
                    .into());
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
                let parent = self.resolve_expr(*parent, None)?;
                let HirExpression { ref ty, .. } = parent;
                match self.types_module.get_type(ty) {
                    HirType::Reference { .. } => {
                        if let Some(index) = self
                            .declarations_module
                            .retrieve_object_body(*ty)
                            .expect("Object should have been defined")
                            .iter()
                            .position(|struct_field| {
                                &self.symbols_module.intern(&field) == struct_field
                            })
                        {
                            let ty = self
                                .types_module
                                .insert_unnamed_type(HirType::Field(FieldMethod::Type(*ty, index)));
                            Ok(HirExpression {
                                id: ExpressionId::new(), // Changed to ExpressionId
                                ty,
                                kind: HirExpressionKind::FieldAccess {
                                    expr: Box::new(parent),
                                    field_index: index,
                                },
                                span: expr.span,
                            })
                        } else {
                            Err(HIRError {
                                kind: HIRErrorKind::PropertyNotRecognized {
                                    prop_names: vec![field],
                                },
                                span: expr.span,
                            }
                            .into())
                        }
                    }
                    HirType::VarReference(rf) => {
                        let ty = self.types_module.insert_unnamed_type(HirType::Field(
                            FieldMethod::Variable(*rf, self.symbols_module.intern(&field)),
                        ));
                        Ok(HirExpression {
                            id: ExpressionId::new(), // Changed to ExpressionId
                            ty,
                            kind: HirExpressionKind::FieldAccess {
                                expr: Box::new(parent),
                                field_index: usize::MAX,
                            },
                            span: expr.span,
                        })
                    }
                    u => unreachable!("{u:?}"),
                }
            }
        }
    }
    pub fn resolve_binary(
        &mut self,
        lhs: ASTExpression,
        op: Operator,
        rhs: ASTExpression,
        ty: Option<TypeId>,
    ) -> Result<HirExpression> {
        let mut lhs = self.resolve_expr(lhs, ty)?;
        let mut rhs = self.resolve_expr(rhs, ty)?;
        if discriminant(self.types_module.get_type(&lhs.ty))
            != discriminant(self.types_module.get_type(&rhs.ty))
        {
            if lhs.ty == self.types_module.infer_id() {
                lhs.ty = rhs.ty;
            } else if rhs.ty == self.types_module.infer_id() {
                rhs.ty = lhs.ty;
            }
        }
        let span = Span {
            start: lhs.span.start,
            end: lhs.span.end,
        };
        Ok(HirExpression {
            ty: match op {
                Operator::Add | Operator::Sub | Operator::Star | Operator::Slash => lhs.ty,
                Operator::Equals
                | Operator::LogicAnd
                | Operator::LogicOr
                | Operator::GreaterThan
                | Operator::GreaterThanOrEqual
                | Operator::LessThan
                | Operator::LessThanOrEqual => self.types_module.bool_id(),
            },
            kind: HirExpressionKind::Binary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            },
            id: ExpressionId::new(), // Changed to ExpressionId
            span,
        })
    }
    /// Creates an int expression that must be inferred.
    pub fn create_intexpr(&self, i: i32, span: Span) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Int(i),
            id: ExpressionId::new(),
            ty: self.types_module.infer_id(),
            span,
        }
    }

    /// Creates a float expression.
    pub fn create_floatexpr(&self, float: f32, span: Span) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Float(float),
            id: ExpressionId::new(),
            ty: self.types_module.float_id(),
            span,
        }
    }
}
