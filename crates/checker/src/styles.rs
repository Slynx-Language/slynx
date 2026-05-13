//* File specific to implement styles resolving, hoisting, and statment resolval

use slynx_hir::model::{
    HirDeclaration, HirDeclarationKind, HirStyleBlock, HirStyleStatement, HirStyleUsage,
    StylesDefinition,
};

use crate::{Result, TypeChecker, error::TypeError};

impl TypeChecker {
    pub fn check_style_statement(&mut self, statement: &mut HirStyleStatement) -> Result<()> {
        let void = self.types_module.void_id();
        match statement {
            HirStyleStatement::Statement(statement) => self.resolve_statement(statement, &void),
            HirStyleStatement::Styles(blocks) => {
                for block in blocks {
                    for def in &mut block.definitions {
                        def.expr.ty = self.unify(&def.expected_type, &def.expr.ty, &def.span)?;
                    }
                }
                Ok(())
            }
        }
    }
    pub fn check_style_usage(&mut self, usage: &mut HirStyleUsage) -> Result<()> {
        let decl_ty = self.declarations[usage.style.as_raw() as usize];
        let params_types = usage
            .params
            .iter_mut()
            .map(|param| self.get_type_of_expr(param))
            .collect::<Result<Vec<_>>>()?;
        let slynx_hir::model::HirType::Style { args } = self.types_module.get_type(&decl_ty) else {
            unreachable!("Type of style should be Style");
        };
        let args = args.clone();
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
            unimplemented!("check stylesheet should receive a stylesheet");
        };
        for usage in usages {
            self.check_style_usage(usage)?;
        }
        for statement in statements {
            self.check_style_statement(statement)?;
        }
        Ok(())
    }

    ///Sets the default value on the given `definition` style.
    pub fn default_style_definition(&mut self, definition: &mut StylesDefinition) -> Result<()> {
        self.default_expr(&mut definition.expr)?;
        definition.expr.ty = self.unify(
            &definition.expr.ty,
            &definition.expected_type,
            &definition.span,
        )?;
        Ok(())
    }
    ///Defaults all the definitions on the given `block`
    pub fn default_style_block(&mut self, block: &mut HirStyleBlock) -> Result<()> {
        for definition in &mut block.definitions {
            self.default_style_definition(definition)?;
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
}
