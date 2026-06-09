//* File specific to implement styles resolving, hoisting, and statment resolval

use slynx_hir::{HirDeclaration, HirDeclarationKind, HirStyleStatement, HirStyleUsage, HirType};

use crate::{Result, TypeChecker, TypeError};

impl TypeChecker {
    pub fn check_style_statement(&mut self, statement: &mut HirStyleStatement) -> Result<()> {
        let void = self.types_module.void_id();
        match statement {
            HirStyleStatement::Statement(statement) => self.resolve_statement(statement, &void),
            HirStyleStatement::Styles(blocks) => {
                for block in blocks {
                    for def in &mut block.definitions {
                        let calc = self.get_type_of_expr(&mut def.expr)?;
                        def.expr.ty = self.unify(&def.expected_type, &calc, &def.span)?;
                    }
                }
                Ok(())
            }
        }
    }
    pub fn resolve_style_usage(&mut self, style_usage: &mut HirStyleUsage) -> Result<()> {
        let args = {
            let reader = self
                .types_module
                .get_type(&self.decl_types[&style_usage.style]);
            let HirType::Style { args } = &*reader else {
                unreachable!("Type of style should be style");
            };
            args.clone()
        };
        for (idx, param) in style_usage.params.iter_mut().enumerate() {
            param.ty = self.unify(&args[idx], &param.ty, &param.span)?;
        }

        Ok(())
    }

    pub fn check_style_usage(&mut self, usage: &mut HirStyleUsage) -> Result<()> {
        let decl_ty = self.decl_types[&usage.style];
        let params_types = usage
            .params
            .iter_mut()
            .map(|param| self.get_type_of_expr(param))
            .collect::<Result<Vec<_>>>()?;
        let args = {
            let reader = self.types_module.get_type(&decl_ty);
            let slynx_hir::HirType::Style { args } = &*reader else {
                unreachable!("Type of style should be Style");
            };
            args.clone()
        };
        if usage.params.len() != args.len() {
            return Err(TypeError::invalid_funcall_args(
                args.len(),
                usage.params.len(),
                usage.span,
            ));
        }
        for (idx, paramty) in params_types.iter().enumerate() {
            let arg = &args[idx];
            let span = usage.params[idx].span;
            usage.params[idx].ty = self.unify(paramty, arg, &span)?;
        }

        Ok(())
    }

    pub fn check_stylesheet(&mut self, style: &mut HirDeclaration) -> Result<()> {
        let HirDeclarationKind::StyleSheet {
            ref mut statements,
            ref mut usages,
            ..
        } = style.kind
        else {
            unreachable!("check stylesheet should receive a stylesheet");
        };
        for usage in usages {
            self.check_style_usage(usage)?;
        }
        for statement in statements {
            self.check_style_statement(statement)?;
        }
        Ok(())
    }
}
