use color_eyre::eyre::Result;

use super::TypeChecker;

use crate::checker::error::{TypeError, TypeErrorKind};
use crate::hir::{
    DeclarationId, TypeId,
    definitions::{
        ComponentMemberDeclaration, HirExpression, HirExpressionKind, HirStatement,
        HirStatementKind, SpecializedComponent,
    },
    types::{FieldMethod, HirType},
};
use common::ast::Span;
impl TypeChecker {
    fn get_function_signature(
        &self,
        declaration: DeclarationId,
        span: &Span,
    ) -> Result<(Vec<TypeId>, TypeId)> {
        let Some(function_ty) = self
            .declarations
            .get(declaration.as_raw() as usize)
            .copied()
        else {
            return Err(TypeError {
                kind: TypeErrorKind::Unrecognized,
                span: span.clone(),
            }
            .into());
        };

        let resolved = self.types_module.get_type(&function_ty).clone();
        let HirType::Function { args, return_type } = resolved.clone() else {
            return Err(TypeError {
                kind: TypeErrorKind::InvalidFunctionCallTarget {
                    declaration,
                    received: resolved,
                },
                span: span.clone(),
            }
            .into());
        };
        Ok((args, return_type))
    }

    fn check_function_call_len(
        &self,
        args: &[HirExpression],
        expected_args: &[TypeId],
        span: &Span,
    ) -> Result<()> {
        if args.len() != expected_args.len() {
            return Err(TypeError {
                kind: TypeErrorKind::InvalidFuncallArgLength {
                    expected_length: expected_args.len(),
                    received_length: args.len(),
                },
                span: span.clone(),
            }
            .into());
        }
        Ok(())
    }

    fn check_function_call_args(
        &mut self,
        args: &mut [HirExpression],
        expected: &[TypeId],
    ) -> Result<()> {
        debug_assert_eq!(
            args.len(),
            expected.len(),
            "arity must be checked before arg validation"
        );
        for (arg, expected_ty) in args.iter_mut().zip(expected.iter().copied()) {
            arg.ty = self.get_type_of_expr(arg)?;
            arg.ty = self.unify(&arg.ty, &expected_ty, &arg.span)?;
        }
        Ok(())
    }

    fn default_function_call_args(
        &mut self,
        args: &mut [HirExpression],
        expected: &[TypeId],
    ) -> Result<()> {
        debug_assert_eq!(
            args.len(),
            expected.len(),
            "arity must be checked before arg defaulting"
        );
        for (arg, expected_ty) in args.iter_mut().zip(expected.iter().copied()) {
            self.default_expr(arg)?;
            arg.ty = self.unify(&arg.ty, &expected_ty, &arg.span)?;
        }
        Ok(())
    }

    pub(super) fn resolve_specialized(&mut self, spec: &mut SpecializedComponent) -> Result<()> {
        match spec {
            SpecializedComponent::Text { text } => {
                text.ty = self.get_type_of_expr(text)?;
                text.ty = self.unify(&text.ty, &self.types_module.str_id(), &text.span)?;
            }
            SpecializedComponent::Div { children } => {
                for child in children {
                    match child {
                        ComponentMemberDeclaration::Property { value, .. } => {
                            if let Some(value) = value {
                                value.ty = self.get_type_of_expr(value)?;
                            }
                        }
                        ComponentMemberDeclaration::Specialized(spec) => {
                            self.resolve_specialized(spec)?;
                        }
                        ComponentMemberDeclaration::Child { name, values, .. } => {
                            self.resolve_component_members(values, *name)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn resolve_statments(
        &mut self,
        statments: &mut Vec<HirStatement>,
        ty: &TypeId,
    ) -> Result<()> {
        let HirType::Function { return_type, .. } = self.types_module.get_type(ty).clone() else {
            unreachable!();
        };
        for statment in statments {
            match &mut statment.kind {
                HirStatementKind::While { condition, body } => {
                    self.default_expr(condition)?;

                    for stmt in body {
                        self.default_statement(stmt, &return_type)?;
                    }
                }

                HirStatementKind::Variable { value, .. } => {
                    value.ty = self.get_type_of_expr(value)?;
                }
                HirStatementKind::Return { expr } => {
                    expr.ty = self.get_type_of_expr(expr)?;
                    expr.ty = self.unify(&expr.ty, &return_type, &statment.span)?;
                }
                HirStatementKind::Expression { expr } => {
                    expr.ty = self.get_type_of_expr(expr)?;
                }
                HirStatementKind::Assign { lhs, value } => {
                    let refty = match self.types_module.get_type(&lhs.ty) {
                        HirType::Field(FieldMethod::Type(_, _)) => lhs.ty,
                        HirType::Field(FieldMethod::Variable(v, name)) => {
                            let object_ty =
                                *self.types_module.get_variable(v).ok_or(TypeError {
                                    kind: TypeErrorKind::Unrecognized,
                                    span: lhs.span.clone(),
                                })?;
                            let HirType::Reference { rf, .. } =
                                self.retrieve_reference_of(v, &lhs.span)?
                            else {
                                unreachable!();
                            };
                            if let Some(index) = self
                                .structs
                                .get(&object_ty)
                                .expect("Type should be defined")
                                .iter()
                                .position(|f| f == name)
                            {
                                let HirExpressionKind::FieldAccess {
                                    ref mut field_index,
                                    ..
                                } = lhs.kind
                                else {
                                    unreachable!();
                                };
                                *field_index = index;
                                *self.types_module.get_type_mut(&lhs.ty) =
                                    HirType::Field(FieldMethod::Type(rf, index));
                                lhs.ty
                            } else {
                                return Err(TypeError {
                                    kind: TypeErrorKind::Unrecognized,
                                    span: lhs.span.clone(),
                                }
                                .into());
                            }
                        }
                        HirType::VarReference(_) => lhs.ty,
                        _ => unreachable!(),
                    };

                    let ty = self.resolve(&lhs.ty, &statment.span)?;
                    lhs.ty = refty;
                    value.ty = self.get_type_of_expr(value)?;
                    value.ty = self.unify(&ty, &value.ty, &value.span)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn resolve_object_types(
        &mut self,
        ty: &HirType,
        fields: &mut [HirExpression],
    ) -> Result<()> {
        let HirType::Struct { fields: fields_tys } = ty else {
            unreachable!("When resolving object types, a type 'struct' should be provided");
        };
        for (idx, f) in fields.iter_mut().enumerate() {
            f.ty = self.unify(&fields_tys[idx], &f.ty, &f.span)?;
        }
        Ok(())
    }

    pub fn get_type_from_ref(&self, ref_ty: TypeId) -> &HirType {
        if let HirType::Reference { rf, .. } = self.types_module.get_type(&ref_ty) {
            self.types_module.get_type(rf)
        } else {
            unreachable!("The provided ref_ty should be of type Reference");
        }
    }

    /// Retrieves the type of the provided `expr`. Returns infer if it could not be inferred.
    pub(super) fn get_type_of_expr(&mut self, expr: &mut HirExpression) -> Result<TypeId> {
        let expected = expr.ty;

        let calc = match expr.kind {
            HirExpressionKind::If {
                ref mut condition,
                ref mut else_branch,
                ref mut then_branch,
            } => {
                let if_ty = if !then_branch.is_empty() {
                    let lasted_index = then_branch.len() - 1;
                    for stmt in &mut then_branch[..lasted_index] {
                        self.default_statement(stmt, &expected)?;
                    }
                    match &mut then_branch[lasted_index].kind {
                        HirStatementKind::Expression { expr } => self.get_type_of_expr(expr)?,
                        _ => self.types_module.void_id(),
                    }
                } else {
                    self.types_module.void_id()
                };
                let else_ty = if let Some(else_branch) = else_branch {
                    if !else_branch.is_empty() {
                        let lasted_index = else_branch.len() - 1;
                        for stmt in &mut else_branch[..lasted_index] {
                            self.default_statement(stmt, &expected)?;
                        }
                        match &mut else_branch[lasted_index].kind {
                            HirStatementKind::Expression { expr } => self.get_type_of_expr(expr)?,
                            _ => self.types_module.void_id(),
                        }
                    } else {
                        self.types_module.void_id()
                    }
                } else {
                    self.types_module.void_id()
                };

                let result_ty = self.unify(&if_ty, &else_ty, &expr.span)?;
                condition.ty =
                    self.unify(&condition.ty, &self.types_module.bool_id(), &condition.span)?;
                result_ty
            }
            HirExpressionKind::FunctionCall {
                name,
                args: ref mut f_args,
            } => {
                let (args, return_type) = self.get_function_signature(name, &expr.span)?;
                self.check_function_call_len(f_args, &args, &expr.span)?;
                self.check_function_call_args(f_args, &args)?;
                return_type
            }
            HirExpressionKind::Int(_) => self.types_module.int_id(),
            HirExpressionKind::Float(_) => self.types_module.float_id(),
            HirExpressionKind::StringLiteral(_) => self.types_module.str_id(),
            HirExpressionKind::Binary {
                ref mut lhs,
                ref mut rhs,
                ref op,
            } => {
                let lhs_ty = self.get_type_of_expr(lhs)?;
                let rhs_ty = self.get_type_of_expr(rhs)?;
                let out = self.unify(&lhs_ty, &rhs_ty, &expr.span)?;
                if op.is_logical() {
                    self.types_module.bool_id()
                } else {
                    out
                }
            }
            HirExpressionKind::Identifier(_) => self.resolve(&expr.ty, &expr.span)?,
            HirExpressionKind::Component {
                name,
                ref mut values,
            } => self.resolve_component_members(values, name)?,
            HirExpressionKind::Object {
                name,
                ref mut fields,
            } => {
                let obj = self.get_type_from_ref(name).clone();
                self.resolve_object_types(&obj, fields)?;
                name
            }

            HirExpressionKind::FieldAccess {
                ref mut field_index,
                expr: ref mut e,
            } => {
                self.get_type_of_expr(e)?;

                match self.types_module.get_type(&expr.ty) {
                    HirType::Field(FieldMethod::Variable(id, name)) => {
                        let object_ty = *self.types_module.get_variable(id).ok_or(TypeError {
                            kind: TypeErrorKind::Unrecognized,
                            span: expr.span.clone(),
                        })?;
                        let HirType::Reference { rf, .. } =
                            self.retrieve_reference_of(id, &expr.span)?
                        else {
                            unreachable!();
                        };
                        if let Some(index) = self
                            .structs
                            .get(&object_ty)
                            .unwrap()
                            .iter()
                            .position(|field| field == name)
                        {
                            *field_index = index;
                            let HirType::Struct { fields } = self.types_module.get_type(&rf) else {
                                unreachable!()
                            };
                            fields[index]
                        } else {
                            return Err(TypeError {
                                kind: TypeErrorKind::Unrecognized,
                                span: expr.span.clone(),
                            }
                            .into());
                        }
                    }
                    _ => {
                        return Err(TypeError {
                            kind: TypeErrorKind::Unrecognized,
                            span: expr.span.clone(),
                        }
                        .into());
                    }
                }
            }
            HirExpressionKind::Bool(_) => self.types_module.bool_id(),
            HirExpressionKind::Specialized(ref mut s) => {
                self.resolve_specialized(s)?;
                self.types_module.generic_component_id()
            }
        };

        expr.ty = self.unify(&expected, &calc, &expr.span)?;
        Ok(expr.ty)
    }

    pub(super) fn default_expr(&mut self, expr: &mut HirExpression) -> Result<()> {
        match expr.kind {
            HirExpressionKind::If {
                ref mut condition,
                ref mut then_branch,
                ref mut else_branch,
            } => {
                self.default_expr(condition)?;
                let unify =
                    self.unify(&condition.ty, &self.types_module.bool_id(), &condition.span)?;
                condition.ty = unify;
                let if_ty = if !then_branch.is_empty() {
                    let lasted_index = then_branch.len() - 1;
                    for stmt in &mut then_branch[..lasted_index] {
                        self.default_statement(stmt, &expr.ty)?;
                    }
                    match &mut then_branch[lasted_index].kind {
                        HirStatementKind::Expression { expr } => self.get_type_of_expr(expr)?,
                        _ => self.types_module.void_id(),
                    }
                } else {
                    self.types_module.void_id()
                };
                let else_ty = if let Some(else_branch) = else_branch {
                    if !else_branch.is_empty() {
                        let lasted_index = else_branch.len() - 1;
                        for stmt in &mut else_branch[..lasted_index] {
                            self.default_statement(stmt, &expr.ty)?;
                        }
                        match &mut else_branch[lasted_index].kind {
                            HirStatementKind::Expression { expr } => self.get_type_of_expr(expr)?,
                            _ => self.types_module.void_id(),
                        }
                    } else {
                        self.types_module.void_id()
                    }
                } else {
                    self.types_module.void_id()
                };

                self.unify(&if_ty, &else_ty, &expr.span)?;
            }
            HirExpressionKind::FunctionCall {
                name,
                args: ref mut f_args,
            } => {
                let (args, return_type) = self.get_function_signature(name, &expr.span)?;
                self.check_function_call_len(f_args, &args, &expr.span)?;
                self.default_function_call_args(f_args, &args)?;
                expr.ty = self.unify(&expr.ty, &return_type, &expr.span)?;
            }
            HirExpressionKind::Bool(_) => {
                expr.ty = self.unify(&expr.ty, &self.types_module.bool_id(), &expr.span)?
            }
            HirExpressionKind::StringLiteral(_) => {
                expr.ty = self.unify(&expr.ty, &self.types_module.str_id(), &expr.span)?
            }
            HirExpressionKind::Int(_) => {
                expr.ty = self.unify(&expr.ty, &self.types_module.int_id(), &expr.span)?
            }
            HirExpressionKind::Float(_) => {
                expr.ty = self.unify(&expr.ty, &self.types_module.float_id(), &expr.span)?
            }
            HirExpressionKind::Binary {
                ref mut lhs,
                ref mut rhs,
                op,
            } => {
                self.default_expr(rhs)?;
                self.default_expr(lhs)?;
                if op.is_logical() {
                    expr.ty = self.types_module.bool_id();
                } else {
                    expr.ty = self.unify(&rhs.ty, &lhs.ty, &expr.span)?;
                }
            }
            HirExpressionKind::Identifier(_) => {
                expr.ty = self.resolve(&expr.ty, &expr.span)?;
            }
            HirExpressionKind::Specialized(ref mut spec) => {
                self.resolve_specialized(spec)?;
                expr.ty = self.unify(
                    &expr.ty,
                    &self.types_module.generic_component_id(),
                    &expr.span,
                )?
            }
            HirExpressionKind::Object { .. } => {
                expr.ty = self.resolve(&expr.ty, &expr.span)?;
            }
            HirExpressionKind::FieldAccess {
                expr: ref mut parent,
                ..
            } => {
                parent.ty = self.resolve(&parent.ty, &expr.span)?;
                expr.ty = self.resolve(&expr.ty, &expr.span)?;
            }
            HirExpressionKind::Component {
                ref name,
                ref mut values,
            } => {
                let ty = self.types_module.insert_unnamed_type(HirType::Reference {
                    rf: *name,
                    generics: Vec::new(),
                });
                expr.ty = ty;
                for val in values {
                    match val {
                        ComponentMemberDeclaration::Property {
                            index, value, span, ..
                        } => {
                            if let Some(value) = value {
                                let ty = self.get_type_of_expr(value)?;
                                let resolved = self.resolve(&expr.ty, &expr.span)?;
                                let HirType::Component { props } =
                                    self.types_module.get_type(&resolved)
                                else {
                                    unreachable!(
                                        "Component expression should be of type component"
                                    );
                                };
                                let propty = props[*index].2;
                                let ty = self.unify(&propty, &ty, span)?;

                                let HirType::Component { props } =
                                    self.types_module.get_type_mut(&resolved)
                                else {
                                    unreachable!(
                                        "Component expression should be of type component"
                                    );
                                };
                                props[*index].2 = ty;
                            }
                        }
                        ComponentMemberDeclaration::Specialized(_) => {}
                        ComponentMemberDeclaration::Child { name, values, .. } => {
                            self.resolve_component_members(values, *name)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
