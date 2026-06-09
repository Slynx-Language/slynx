use crate::{
    HIRError, Result, SlynxHir, TypeId,
    model::{
        HirStatement, HirStatementKind, HirStyleBlock, HirStyleBlockKind, HirStyleStatement,
        StylesDefinition,
    },
    module_loader::FileId,
};
use common::Span;
use slynx_parser::{ASTStatement, ASTStatementKind, NamedExpr, StyleBlock, StyleSheetStatement};

impl SlynxHir {
    /// Resolves an AST statement into a typed [`HirStatement`].
    pub(crate) fn resolve_statement(
        &mut self,
        fileid: FileId,
        statement: &ASTStatement,
    ) -> Result<HirStatement> {
        match &statement.kind {
            ASTStatementKind::Expression(expr) => {
                let expr = self.generate_expression(fileid, expr, None)?;
                Ok(HirStatement::new_expression(expr))
            }
            ASTStatementKind::Assign { lhs, rhs } => {
                let lhs = self.generate_expression(fileid, lhs, None)?;
                let rhs = self.generate_expression(fileid, rhs, None)?;

                Ok(HirStatement {
                    kind: HirStatementKind::Assign { lhs, value: rhs },
                    span: statement.span,
                })
            }
            kind @ (ASTStatementKind::MutableVar { name, ty, rhs }
            | ASTStatementKind::Var { name, ty, rhs }) => {
                let typeid = ty.as_ref().and_then(|t| {
                    let symbol = self.intern_name(&t.identifier);
                    self.get_type_of_name(symbol, &statement.span).ok()
                });
                let name = self.intern_name(name);
                let rhs = self.generate_expression(fileid, rhs, typeid)?;
                let id = if let ASTStatementKind::MutableVar { .. } = kind {
                    self.create_mutable_variable(fileid, name, rhs.ty, &statement.span)
                } else {
                    self.create_variable(fileid, name, rhs.ty, &statement.span)
                }?;

                Ok(HirStatement::new_variable(id, rhs, statement.span))
            }

            ASTStatementKind::While { condition, body } => {
                let condition = self.generate_expression(fileid, condition, None)?;
                let body = body
                    .iter()
                    .map(|stmt| self.resolve_statement(fileid, stmt))
                    .collect::<Result<Vec<_>>>()?;
                Ok(HirStatement::new_while(condition, body, statement.span))
            }
        }
    }

    ///Transforms the given `statement` into an HIR style statement
    pub(crate) fn resolve_stylesheet_statement(
        &mut self,
        fileid: FileId,
        statement: &StyleSheetStatement,
    ) -> Result<HirStyleStatement> {
        match statement {
            StyleSheetStatement::Statement(s) => self
                .resolve_statement(fileid, s)
                .map(|s| HirStyleStatement::Statement(Box::new(s))),
            StyleSheetStatement::Styles { styles, .. } => self
                .resolve_stylesblock(fileid, styles)
                .map(HirStyleStatement::Styles),
        }
    }

    ///Type of styles is made by its name.
    pub(crate) fn resolve_style_type(&mut self, name: &str, span: Span) -> Result<TypeId> {
        let ty = match name {
            "backgroundColor" | "foregroundColor" => self.int32_type(),
            _ => {
                let name = self.intern_name(name);
                return Err(HIRError::invalid_style_definition(name, span));
            }
        };
        Ok(ty)
    }

    ///Transforms the given `definitions` in a vector of `StyleDefinition`
    pub(crate) fn resolve_style_definitions(
        &mut self,
        fileid: FileId,
        definitions: &[NamedExpr],
    ) -> Result<Vec<StylesDefinition>> {
        definitions
            .iter()
            .map(|def| {
                let expr = self.generate_expression(fileid, &def.expr, None)?;
                let expected_type = self.resolve_style_type(&def.name, def.span)?;
                let symbol = self.intern_name(&def.name);
                Ok(StylesDefinition::new(symbol, expr, expected_type, def.span))
            })
            .collect::<Result<Vec<_>>>()
    }

    ///Resolves the given `styles` blocks, and creates `HirStyleBlock`s based on them
    pub(crate) fn resolve_stylesblock(
        &mut self,
        fileid: FileId,
        styles: &[StyleBlock],
    ) -> Result<Vec<HirStyleBlock>> {
        let mut out = Vec::new();
        for style in styles {
            let kind = {
                let states = &style.state.states;
                if states.iter().any(|s| s == "hover") {
                    HirStyleBlockKind::Hover
                } else if states.is_empty() || states.iter().any(|s| s == "default") {
                    HirStyleBlockKind::Default
                } else {
                    let event = self.intern_name(&states[0]);
                    return Err(HIRError::invalid_style_event(event, style.span));
                }
            };
            let definitions = self.resolve_style_definitions(fileid, &style.properties)?;
            out.push(HirStyleBlock { kind, definitions });
        }
        Ok(out)
    }
}
