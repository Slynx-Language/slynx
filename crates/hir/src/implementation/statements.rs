use crate::{
    HIRError, Result, SlynxHir, TypeId,
    model::{
        HirStatement, HirStatementKind, HirStyleBlock, HirStyleBlockKind, HirStyleStatement,
        StylesDefinition,
    },
};
use common::Span;
use slynx_parser::{
    ASTExpression, ASTExpressionKind, ASTStatement, ASTStatementKind, NamedExpr, StyleBlock,
    StyleSheetStatement,
};

impl SlynxHir {
    /// Checks that the given expression refers to an already-defined name, returning an error if not.
    pub fn check_existance(&mut self, expr: &ASTExpression) -> Result<()> {
        match &expr.kind {
            ASTExpressionKind::FieldAccess { parent, .. } => {
                self.check_existance(parent)?;
            }
            ASTExpressionKind::TupleAccess { tuple, .. } => {
                self.check_existance(tuple)?;
            }
            ASTExpressionKind::Identifier(name) => {
                let name = self.modules.intern_name(name);
                self.get_variable(name, &expr.span)?;
            }
            _ => {}
        }
        Ok(())
    }
    /// Resolves an AST statement into a typed [`HirStatement`].
    pub fn resolve_statement(&mut self, statement: &ASTStatement) -> Result<HirStatement> {
        match &statement.kind {
            ASTStatementKind::Expression(expr) => {
                let expr = self.resolve_expr(expr, None)?;
                Ok(HirStatement::new_expression(expr))
            }
            ASTStatementKind::Assign { lhs, rhs } => {
                let lhs = self.resolve_expr(lhs, None)?;
                let rhs = self.resolve_expr(rhs, None)?;

                Ok(HirStatement {
                    kind: HirStatementKind::Assign { lhs, value: rhs },
                    span: statement.span,
                })
            }
            ASTStatementKind::MutableVar { name, ty, rhs } => {
                let typeid = ty.as_ref().and_then(|t| {
                    self.retrieve_information_of_type(&t.identifier, &statement.span)
                        .ok()
                        .map(|inner| inner.0)
                });
                let rhs = self.resolve_expr(rhs, typeid)?;
                let name = self.modules.intern_name(name);
                let id = self.create_mutable_variable(name, rhs.ty, &statement.span)?;

                Ok(HirStatement::new_variable(id, rhs, statement.span))
            }
            ASTStatementKind::Var { name, ty, rhs } => {
                let typeid = ty.as_ref().and_then(|t| {
                    self.retrieve_information_of_type(&t.identifier, &statement.span)
                        .ok()
                        .map(|inner| inner.0)
                });
                let rhs = self.resolve_expr(rhs, typeid)?;
                let name = self.modules.intern_name(name);
                let id = self.create_variable(name, rhs.ty, &statement.span)?;

                Ok(HirStatement::new_variable(id, rhs, statement.span))
            }

            ASTStatementKind::While { condition, body } => {
                let condition = self.resolve_expr(condition, None)?;
                let body = body
                    .iter()
                    .map(|stmt| self.resolve_statement(stmt))
                    .collect::<Result<Vec<_>>>()?;
                Ok(HirStatement::new_while(condition, body, statement.span))
            }
        }
    }

    ///Transforms the given `statement` into an HIR style statement
    pub fn resolve_stylesheet_statement(
        &mut self,
        statement: &StyleSheetStatement,
    ) -> Result<HirStyleStatement> {
        match statement {
            StyleSheetStatement::Statement(s) => {
                self.resolve_statement(s).map(HirStyleStatement::Statement)
            }
            StyleSheetStatement::Styles { styles, .. } => self
                .resolve_stylesblock(styles)
                .map(HirStyleStatement::Styles),
        }
    }

    ///Type of styles is made by its name.
    pub fn resolve_style_type(&mut self, name: &str, span: Span) -> Result<TypeId> {
        let ty = match name {
            "backgroundColor" | "foregroundColor" => self.int32_type(),
            _ => {
                let name = self.modules.intern_name(name);
                return Err(HIRError::invalid_style_definition(name, span));
            }
        };
        Ok(ty)
    }

    ///Transforms the given `definitions` in a vector of `StyleDefinition`
    pub fn resolve_style_definitions(
        &mut self,
        definitions: &[NamedExpr],
    ) -> Result<Vec<StylesDefinition>> {
        definitions
            .iter()
            .map(|def| {
                let expr = self.resolve_expr(&def.expr, None)?;
                let expected_type = self.resolve_style_type(&def.name, def.span)?;
                let symbol = self.modules.intern_name(&def.name);
                Ok(StylesDefinition::new(symbol, expr, expected_type, def.span))
            })
            .collect::<Result<Vec<_>>>()
    }

    ///Resolves the given `styles` blocks, and creates `HirStyleBlock`s based on them
    pub fn resolve_stylesblock(&mut self, styles: &[StyleBlock]) -> Result<Vec<HirStyleBlock>> {
        let mut out = Vec::new();
        for style in styles {
            let kind = match style.event.as_deref() {
                Some("hover") => HirStyleBlockKind::Hover,
                None => HirStyleBlockKind::Default,
                Some(event) => {
                    let event = self.modules.intern_name(event);
                    return Err(HIRError::invalid_style_event(event, style.span));
                }
            };
            let definitions = self.resolve_style_definitions(&style.properties)?;
            out.push(HirStyleBlock { kind, definitions });
        }
        Ok(out)
    }
}
