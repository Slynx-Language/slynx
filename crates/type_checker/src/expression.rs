//! Type checking logic for expressions and statements.
//!
//! This module implements the core type-checking pass for the Slynx HIR.
//! It handles the resolution of function bodies, component property
//! initialization, and ensures type safety through unification.

use super::{Result, TypeChecker};

use crate::{TypeError, TypeErrorKind};
use common::Span;
use slynx_hir::{
    DeclarationId, FieldMethod, HirComponentExpression, HirExpression, HirExpressionKind,
    HirSpecializedComponentExpression, HirType, TypeId,
};
impl TypeChecker {
    /// Resolves the struct type from a reference type.
    ///
    /// This function recursively follows reference types until it finds a struct type.
    /// If the type is not a struct, a `TypeError` is returned.
    pub fn get_struct_from_ref(&self, ty: &TypeId, span: &Span) -> Result<TypeId> {
        match self.types_module.get_type(ty) {
            HirType::Reference { rf, .. } => self.get_struct_from_ref(&rf, span),
            HirType::Struct { .. } => Ok(*ty),
            v => Err(TypeError {
                kind: TypeErrorKind::NotAStruct(v.clone()),
                span: *span,
            }),
        }
    }

    /// Resolves the type of a field access expression.
    ///
    /// This function resolves the type of a field access expression by following
    /// the field method (type or variable) and updating the field index.
    pub(super) fn resolve_field_access_type(
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
                    .create_unnamed_type(HirType::Field(FieldMethod::Type(rf, index)));
                self.resolve(field_ty, span)
            }
            FieldMethod::Tuple(rf, index) => {
                *field_index = index;
                *field_ty = self
                    .types_module
                    .create_unnamed_type(HirType::Field(FieldMethod::Tuple(rf, index)));
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
                    .types_module
                    .objects
                    .get(&layout_ty)
                    .expect("Type should be defined")
                    .iter()
                    .position(|field| *field == field_name)
                else {
                    return Err(TypeError {
                        kind: TypeErrorKind::Unrecognized,
                        span: *span,
                    });
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
    pub(crate) fn get_function_signature(
        &self,
        declaration: DeclarationId,
        span: &Span,
    ) -> Result<(Vec<TypeId>, TypeId)> {
        let Some(function_ty) = self.decl_types.get(&declaration).copied() else {
            return Err(TypeError {
                kind: TypeErrorKind::Unrecognized,
                span: *span,
            });
        };

        let resolved = self.types_module.get_type(&function_ty).clone();
        let HirType::Function { args, return_type } = resolved else {
            return Err(TypeError {
                kind: TypeErrorKind::InvalidFunctionCallTarget {
                    declaration,
                    received: resolved,
                },
                span: *span,
            });
        };
        Ok((args, return_type))
    }
    ///Asserts that `args` has same length as `expected_args`
    pub(crate) fn check_function_call_length(
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
            });
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

    /// Resolves the type of a specialized component expression.
    ///
    /// This function resolves the type of a specialized component expression by
    /// updating the type of the expression and its children. If a type mismatch
    /// occurs, a `TypeError` is returned.
    pub(super) fn resolve_specialized(
        &mut self,
        spec: &mut HirSpecializedComponentExpression,
    ) -> Result<()> {
        match spec {
            HirSpecializedComponentExpression::Text { text, style } => {
                text.ty = self.get_type_of_expr(text)?;
                text.ty = self.unify(&text.ty, &self.types_module.str_id(), &text.span)?;
                if let Some(style) = style {
                    self.resolve_style_usage(style)?;
                }
            }
            HirSpecializedComponentExpression::Div { children, style } => {
                for child in children {
                    self.resolve_component_expression(child)?;
                }
                if let Some(style) = style {
                    self.resolve_style_usage(style)?;
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
            f.ty = self.get_type_of_expr(f)?;
            f.ty = self.unify(&fields_tys[idx], &f.ty, &f.span)?;
        }

        Ok(())
    }

    /// Resolves the type of a reference expression, returning the type it references.
    pub fn get_type_from_ref(&self, ref_ty: TypeId, span: Span) -> Result<TypeId> {
        match self.types_module.get_type_from_ref(ref_ty, &span) {
            Ok(v) => Ok(v),
            Err(_) => {
                let ty = self.types_module.get_type(&ref_ty).clone();
                Err(TypeError {
                    kind: TypeErrorKind::CyclicType { ty },
                    span,
                })
            }
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
                self.types_module.create_tuple_type(field_types)
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

                self.check_function_call_length(f_args, &args, &expr.span)?;
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
            HirExpressionKind::Component(HirComponentExpression::Specialized(_)) => {
                self.types_module.generic_component_id()
            }
            HirExpressionKind::Component(HirComponentExpression::Normal { name, .. }) => name,
            HirExpressionKind::Object {
                name,
                ref mut fields,
            } => {
                let obj = self.get_type_from_ref(name, expr.span)?;
                let obj = self.types_module.get_type(&obj).clone();
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
        };

        expr.ty = if expected == calc {
            calc
        } else {
            self.unify(&expected, &calc, &expr.span)?
        };
        Ok(expr.ty)
    }
}
