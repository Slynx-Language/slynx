//! Type checking logic for expressions and statements.
//!
//! This module implements the core type-checking pass for the Slynx HIR.
//! It handles the resolution of function bodies, component property
//! initialization, and ensures type safety through unification.

use color_eyre::eyre::Result;

use super::TypeChecker;

use crate::checker::error::{TypeError, TypeErrorKind};
use crate::hir::{
    DeclarationId, TypeId,
    model::{
        ComponentMemberDeclaration, FieldMethod, HirExpression, HirExpressionKind, HirStatement,
        HirStatementKind, HirType, SpecializedComponent,
    },
};
use common::ast::Span;
impl TypeChecker {
    /// Resolves the struct type from a reference type.
    ///
    /// This function recursively follows reference types until it finds a struct type.
    /// If the type is not a struct, a `TypeError` is returned.
    pub fn get_struct_from_ref(&self, ty: &TypeId, span: &Span) -> Result<TypeId> {
        match self.types_module.get_type(ty) {
            HirType::Reference { rf, .. } => self.get_struct_from_ref(rf, span),
            HirType::Struct { .. } => Ok(*ty),
            v => Err(TypeError {
                kind: TypeErrorKind::NotAStruct(v.clone()),
                span: *span,
            }
            .into()),
        }
    }

    /// Resolves the type of a field access expression.
    ///
    /// This function resolves the type of a field access expression by following
    /// the field method (type or variable) and updating the field index.
    fn resolve_field_access_type(
        &mut self,
        field_ty: &mut TypeId,
        field_index: &mut usize,
        span: &Span,
    ) -> Result<TypeId> {
        let HirType::Field(field_method) = self.types_module.get_type(field_ty).clone() else {
            return self.resolve(field_ty, span);
        };
        match field_method {
            FieldMethod::Type(rf, index) => {
                *field_index = index;
                *field_ty = self
                    .types_module
                    .insert_unnamed_type(HirType::Field(FieldMethod::Type(rf, index)));
                self.resolve(field_ty, span)
            }
            FieldMethod::Tuple(rf, index) => {
                *field_index = index;
                *field_ty = self
                    .types_module
                    .insert_unnamed_type(HirType::Field(FieldMethod::Tuple(rf, index)));
                self.resolve(field_ty, span)
            }
            FieldMethod::Variable(variable_id, field_name) => {
                // Field accesses first enter the checker attached to the source
                // variable name. Resolve that symbolic access once and rewrite it
                // into an indexed field lookup for the rest of the pipeline.
                let object_ty = *self
                    .types_module
                    .get_variable(&variable_id)
                    .ok_or(TypeError {
                        kind: TypeErrorKind::Unrecognized,
                        span: *span,
                    })?;
                let layout_ty = self.get_object_layout_type(&object_ty, span)?;

                let Some(index) = self
                    .structs
                    .get(&layout_ty)
                    .expect("Type should be defined")
                    .iter()
                    .position(|field| *field == field_name)
                else {
                    return Err(TypeError {
                        kind: TypeErrorKind::Unrecognized,
                        span: *span,
                    }
                    .into());
                };

                *field_index = index;
                *self.types_module.get_type_mut(field_ty) =
                    HirType::Field(FieldMethod::Type(object_ty, index));
                self.resolve(field_ty, span)
            }
        }
    }

    /// Gets the signature of the provided `declaration` id expecting its a function type.
    ///
    /// This function retrieves the function type from the declarations and returns
    /// the argument types and return type. If the declaration is not a function,
    /// a `TypeError` is returned.
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
                span: *span,
            }
            .into());
        };

        let resolved = self.types_module.get_type(&function_ty).clone();
        let HirType::Function { args, return_type } = resolved else {
            return Err(TypeError {
                kind: TypeErrorKind::InvalidFunctionCallTarget {
                    declaration,
                    received: resolved,
                },
                span: *span,
            }
            .into());
        };
        Ok((args, return_type))
    }
    ///Asserts that `args` has same length as `expected_args`
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
                span: *span,
            }
            .into());
        }
        Ok(())
    }

    /// Checks that `args` have the same types as `expected`.
    ///
    /// This function checks that each argument in `args` has a type that can be unified with
    /// the corresponding type in `expected`. If a type mismatch occurs, a `TypeError` is returned.
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

    /// Defaults the provided `args` to their expected types.
    ///
    /// This function defaults each argument in `args` to its expected type. If a type mismatch
    /// occurs, a `TypeError` is returned.
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

    /// Resolves the type of a specialized component expression.
    ///
    /// This function resolves the type of a specialized component expression by
    /// updating the type of the expression and its children. If a type mismatch
    /// occurs, a `TypeError` is returned.
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
    /// Resolves a while statement, checking that the condition is a boolean and the body is a sequence of statements.
    fn resolve_statement_while(
        &mut self,
        condition: &mut HirExpression,
        body: &mut [HirStatement],
        ty: &TypeId,
    ) -> Result<()> {
        condition.ty = self.get_type_of_expr(condition)?;
        let bool_id = self.types_module.bool_id();

        condition.ty = self.unify(&condition.ty, &bool_id, &condition.span)?;

        self.resolve_statements(body, ty)?;
        Ok(())
    }
    /// Resolves an assignment statement, checking that the types match.
    fn resolve_statement_assign(
        &mut self,
        lhs: &mut HirExpression,
        value: &mut HirExpression,
        span: &Span,
    ) -> Result<()> {
        let refty = match self.types_module.get_type(&lhs.ty) {
            HirType::Field(FieldMethod::Type(_, _)) | HirType::Field(FieldMethod::Tuple(_, _)) => {
                lhs.ty
            }
            HirType::Field(FieldMethod::Variable(..)) => {
                let HirExpressionKind::FieldAccess {
                    ref mut field_index,
                    ..
                } = lhs.kind
                else {
                    unreachable!();
                };
                let _ = self.resolve_field_access_type(&mut lhs.ty, field_index, &lhs.span)?;
                lhs.ty
            }
            HirType::VarReference(_) => lhs.ty,
            _ => unreachable!(),
        };

        let ty = self.resolve(&lhs.ty, span)?;
        lhs.ty = refty;
        value.ty = self.get_type_of_expr(value)?;
        value.ty = self.unify(&ty, &value.ty, &value.span)?;
        Ok(())
    }
    /// Resolves the types of the provided `statements`.
    ///
    /// This function resolves the types of the provided `statements` by updating
    /// the type of each statement and its children. If a type mismatch occurs,
    /// a `TypeError` is returned.
    pub(super) fn resolve_statements(
        &mut self,
        statements: &mut [HirStatement],
        ty: &TypeId,
    ) -> Result<()> {
        let HirType::Function { return_type, .. } = self.types_module.get_type(ty).clone() else {
            unreachable!();
        };
        for statement in statements {
            let span = &statement.span;

            match &mut statement.kind {
                HirStatementKind::While { condition, body } => {
                    self.resolve_statement_while(condition, body, ty)?
                }

                HirStatementKind::Variable { value, .. } => {
                    value.ty = self.get_type_of_expr(value)?;
                }

                HirStatementKind::Return { expr } => {
                    expr.ty = self.get_type_of_expr(expr)?;
                    expr.ty = self.unify(&expr.ty, &return_type, span)?;
                }

                HirStatementKind::Expression { expr } => {
                    expr.ty = self.get_type_of_expr(expr)?;
                }

                HirStatementKind::Assign { lhs, value } => {
                    self.resolve_statement_assign(lhs, value, span)?
                }
            }
        }

        Ok(())
    }

    /// Resolves the types of the provided `fields` asserting that `ty` is a type for a struct,
    /// and the types of `fields` match the type expected by the fields of the given struct type.
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

    /// Resolves the type of a reference expression, returning the type it references.
    pub fn get_type_from_ref(&self, ref_ty: TypeId) -> &HirType {
        if let HirType::Reference { rf, .. } = self.types_module.get_type(&ref_ty) {
            self.types_module.get_type(rf)
        } else {
            unreachable!("The provided ref_ty should be of type Reference");
        }
    }
    /// Helper to resolve a branch (then/else) that may be present or absent.
    /// Accepts `Option<&mut Vec<HirStatement>>` because the else branch is optional
    /// in the HIR representation. `expected` is the expected TypeId for statements.
    fn resolve_branch(
        &mut self,
        branch: Option<&mut Vec<HirStatement>>,
        expected: &TypeId,
    ) -> Result<TypeId> {
        let Some(stmts) = branch else {
            return Ok(self.types_module.void_id());
        };
        let Some((last, rest)) = stmts.split_last_mut() else {
            return Ok(self.types_module.void_id());
        };
        for stmt in rest {
            self.default_statement(stmt, expected)?;
        }
        match &mut last.kind {
            HirStatementKind::Expression { expr } => self.get_type_of_expr(expr),
            _ => Ok(self.types_module.void_id()),
        }
    }

    /// Retrieves the type of the provided `expr`. Returns infer if it could not be inferred.
    pub(super) fn get_type_of_expr(&mut self, expr: &mut HirExpression) -> Result<TypeId> {
        let expected = expr.ty;

        let calc = match expr.kind {
            HirExpressionKind::Tuple(ref mut fields) => {
                let field_types = fields
                    .iter_mut()
                    .map(|f| self.get_type_of_expr(f))
                    .collect::<Result<Vec<_>>>()?;
                self.types_module.add_tuple_type(field_types)
            }
            HirExpressionKind::If {
                ref mut condition,
                ref mut else_branch,
                ref mut then_branch,
            } => {
                let if_ty = self.resolve_branch(Some(then_branch), &expected)?;
                let else_ty = self.resolve_branch(else_branch.as_mut(), &expected)?;

                let result_ty = self.unify(&if_ty, &else_ty, &expr.span)?;
                let cond_ty = self.get_type_of_expr(condition)?;
                condition.ty =
                    self.unify(&cond_ty, &self.types_module.bool_id(), &condition.span)?;

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
                self.resolve_field_access_type(&mut expr.ty, field_index, &expr.span)?
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

    /// Sets the default type on the provided `expr`.
    ///
    /// This function sets the default type on the provided `expr` by recursively
    /// defaulting each field in the tuple and collecting their types. The resulting
    pub(super) fn default_expr(&mut self, expr: &mut HirExpression) -> Result<()> {
        match expr.kind {
            HirExpressionKind::Tuple(ref mut fields) => {
                let types = fields
                    .iter_mut()
                    .map(|f| {
                        self.default_expr(f)?;
                        Ok(f.ty)
                    })
                    .collect::<Result<Vec<_>>>()?;

                let tuple_ty = self.types_module.add_tuple_type(types);

                expr.ty = self.unify(&tuple_ty, &expr.ty, &expr.span)?;
            }
            HirExpressionKind::If {
                ref mut condition,
                ref mut then_branch,
                ref mut else_branch,
            } => {
                self.default_expr(condition)?;
                condition.ty =
                    self.unify(&condition.ty, &self.types_module.bool_id(), &condition.span)?;
                let if_ty = self.resolve_branch(Some(then_branch), &expr.ty)?;
                let else_ty = self.resolve_branch(else_branch.as_mut(), &expr.ty)?;
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
                expr.ty = self.types_module.insert_unnamed_type(HirType::Reference {
                    rf: *name,
                    generics: Vec::new(),
                });

                for val in values {
                    match val {
                        ComponentMemberDeclaration::Property {
                            index, value, span, ..
                        } => {
                            if let Some(value) = value {
                                let expr_ty = self.get_type_of_expr(value)?;
                                let resolved = self.resolve(&expr.ty, &expr.span)?;

                                let HirType::Component {
                                    props: mut declared,
                                } = self.types_module.get_type(&resolved).clone()
                                else {
                                    unreachable!(
                                        "Component expression should be of type component"
                                    );
                                };

                                *declared[*index].prop_type_mut() =
                                    self.unify(declared[*index].prop_type(), &expr_ty, span)?;

                                *self.types_module.get_type_mut(&resolved) =
                                    HirType::Component { props: declared };
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
