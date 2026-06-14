//* File specific for defining default types on declarations
use slynx_hir::{
    HirComponentExpression, HirDeclaration, HirDeclarationKind, HirExpression, HirExpressionKind,
    HirSpecializedComponentExpression, HirStatement, HirStatementKind, HirStyleBlock,
    HirStyleStatement, HirType, StylesDefinition, TypeId,
};

use crate::{Result, TypeChecker};

impl TypeChecker {
    pub fn default_stylesheet(&mut self, decl: &mut HirDeclaration) -> Result<()> {
        let HirDeclarationKind::StyleSheet {
            ref mut statements, ..
        } = decl.kind
        else {
            unreachable!("default_stylesheet should receive a stylesheet declaration");
        };

        for statement in &mut *statements {
            self.default_style_statement(statement)?;
        }
        Ok(())
    }

    ///Asserts that the given `decl` is a function and sets default values to all of its statements, and ensure its returns are correctly.
    ///If the given `decl` is not a function, a panic is going to be received, since it's obviously a bug
    pub fn default_function(&mut self, decl: &mut HirDeclaration) -> Result<()> {
        let HirDeclarationKind::Function {
            ref mut statements, ..
        } = decl.kind
        else {
            unreachable!("default_function should receive a function declaration");
        };
        let HirType::Function { return_type, .. } = self.types_module.get_type(&decl.ty).clone()
        else {
            unreachable!("A function should have function type");
        };
        let infer = self.types_module.infer_id();
        for statement in &mut *statements {
            self.default_statement(statement, &infer)?;
        }
        self.ensure_function_returns(statements, return_type, &decl.span)?;
        Ok(())
    }

    ///Sets the default types on the given `decl`. This replaces the infer types on everything on the given `decl` with the correct(or default) type
    pub fn set_default(&mut self, decl: &mut HirDeclaration) -> Result<()> {
        match decl.kind {
            HirDeclarationKind::Object | HirDeclarationKind::Alias => {}
            HirDeclarationKind::Function { .. } => self.default_function(decl)?,
            HirDeclarationKind::ComponentDeclaration { ref mut props, .. } => {
                self.resolve_component_members(props, decl.ty)?;
            }
            HirDeclarationKind::StyleSheet { .. } => self.default_stylesheet(decl)?,
        }
        Ok(())
    }

    pub fn default_style_statement(&mut self, statement: &mut HirStyleStatement) -> Result<()> {
        match statement {
            HirStyleStatement::Statement(s) => {
                let infer = self.types_module.infer_id();
                self.default_statement(s, &infer)?;
            }
            HirStyleStatement::Styles(styleblocks) => {
                for block in styleblocks {
                    self.default_style_block(block)?;
                }
            }
        }
        Ok(())
    }

    pub fn default_style_block(&mut self, block: &mut HirStyleBlock) -> Result<()> {
        for definition in &mut block.definitions {
            self.default_style_definition(definition)?;
        }
        Ok(())
    }

    pub fn default_style_definition(&mut self, definition: &mut StylesDefinition) -> Result<()> {
        self.default_expr(&mut definition.expr)?;
        definition.expr.ty = self.unify(
            &definition.expr.ty,
            &definition.expected_type,
            &definition.span,
        )?;
        Ok(())
    }
    ///Sets the default value on the given `statement`
    pub fn default_statement(
        &mut self,
        statement: &mut HirStatement,
        expected: &TypeId,
    ) -> Result<()> {
        match &mut statement.kind {
            HirStatementKind::Variable { name, value } => {
                // Ensure the initializer expression is fully typed and propagate it to the variable.
                self.default_expr(value)?;

                let ty = self.get_type_of_expr(value)?;

                value.ty = ty;
                self.types_module.insert_variable(*name, ty);
            }

            HirStatementKind::Assign { lhs, value } => {
                let ty = self.resolve(&lhs.ty, &lhs.span)?;
                value.ty = self.unify(&ty, &value.ty, &value.span)?;
            }

            HirStatementKind::Expression { expr } => self.default_expr(expr)?,

            HirStatementKind::Return { expr } => {
                self.default_expr(expr)?;
                let unify = self.unify(&expr.ty, expected, &statement.span)?;

                expr.ty = unify;
            }

            HirStatementKind::While { condition, body } => {
                condition.ty = self.get_type_of_expr(condition)?;
                let bool_id = self.types_module.bool_id();

                condition.ty = self.unify(&condition.ty, &bool_id, &condition.span)?;

                for stmt in body {
                    self.default_statement(stmt, expected)?;
                }
            }
        };

        Ok(())
    }
    /// Defaults the provided `args` to their expected types.
    ///
    /// This function defaults each argument in `args` to its expected type. If a type mismatch
    /// occurs, a `TypeError` is returned.
    pub(crate) fn default_function_call_args(
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
    pub(super) fn default_component_expression(
        &mut self,
        expr: &mut HirComponentExpression,
    ) -> Result<()> {
        match expr {
            HirComponentExpression::Specialized(spec) => match spec {
                HirSpecializedComponentExpression::Text { text, style } => {
                    self.default_expr(text)?;
                    text.ty = self.unify(&text.ty, &self.types_module.str_id(), &text.span)?;
                    if let Some(style) = style {
                        self.resolve_style_usage(style)?;
                    }
                }
                HirSpecializedComponentExpression::Div { children, style } => {
                    for child in children {
                        self.default_component_expression(child)?;
                    }
                    if let Some(style) = style {
                        self.resolve_style_usage(style)?;
                    }
                }
            },
            HirComponentExpression::Normal {
                name,
                properties,
                children,
                span,
            } => {
                let props = {
                    let HirType::Component { props } = &*self.types_module.get_type(name) else {
                        unreachable!("Should've received a component type");
                    };
                    props.clone()
                };

                for prop_expr in properties {
                    self.default_expr(prop_expr.expr_mut())?;
                    prop_expr.expr_mut().ty = self.unify(
                        props[prop_expr.index()].prop_type(),
                        &prop_expr.expr().ty,
                        span,
                    )?;
                }
                for child in children {
                    self.default_component_expression(child)?;
                }
            }
        }
        Ok(())
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

                let tuple_ty = self.types_module.create_tuple_type(types);

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
                self.check_function_call_length(f_args, &args, &expr.span)?;
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

            HirExpressionKind::Object { .. } => {}
            HirExpressionKind::FieldAccess {
                expr: ref mut parent,
                ..
            } => {
                parent.ty = self.get_type_from_ref(parent.ty, expr.span)?;
                expr.ty = self.resolve(&expr.ty, &expr.span)?;
            }
            HirExpressionKind::Component(ref mut component_expr) => {
                self.default_component_expression(component_expr)?;
                expr.ty = self.resolve(&expr.ty, &expr.span)?;
            }
            HirExpressionKind::MethodCall { .. } => {
                unreachable!(
                    "Method call should have been already removed due to previous type checkings"
                );
            }
        }
        Ok(())
    }
}
