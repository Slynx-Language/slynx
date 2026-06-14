use common::{Span, VisibilityModifier};
use slynx_parser::{
    ASTExpression, ASTExpressionKind, GenericIdentifier, StyleSheetStatement, TypedName,
};

use crate::{
    DeclarationId, HirDeclaration, HirStyleUsage, HirType, Result, SlynxHir, module_loader::FileId,
};

impl SlynxHir {
    ///Hoists a `stylesheet` declaration
    pub(crate) fn hoist_stylesheet(
        &self,
        file: FileId,
        name: &str,
        args: &[TypedName],
        visibility: VisibilityModifier,
    ) -> DeclarationId {
        let name = self.intern_name(name);
        let ty = self.types_module.create_type(
            name,
            HirType::Style {
                args: args.iter().map(|_| self.void_type()).collect(),
            },
        );
        let local = self
            .get_file_mut(file)
            .declarations
            .register_declaration_metadata(name, ty, visibility);
        DeclarationId::new(file, local)
    }

    ///Resolves a `stylesheet` declaration
    pub(crate) fn resolve_stylesheet(
        &self,
        fileid: FileId,
        name: &GenericIdentifier,
        args: &[TypedName],
        usages: &[ASTExpression],
        body: &[StyleSheetStatement],
        span: Span,
    ) -> Result<()> {
        let symbol = self.intern_name(&name.identifier);
        let (id, typeid) = self.find_declaration_by_name(&symbol, name.span)?;

        // Hold write lock only for scope entry.
        self.get_file_mut(fileid).scopes.enter_scope();

        let (args, argsty) = args
            .iter()
            .map(|arg| {
                let symbol = self.intern_name(&arg.name);
                let ty_symbol = self.intern_name(&arg.kind.identifier);
                let ty = self.get_type_of_name(ty_symbol, &arg.kind.span)?;
                self.create_variable(fileid, symbol, ty, &arg.span)
                    .map(|v| (v, ty))
            })
            .collect::<Result<(Vec<_>, Vec<_>)>>()?;
        {
            let mut writer = self.get_type_mut(typeid);
            let HirType::Style { args } = &mut *writer else {
                unreachable!("Type of stylesheet should be style");
            };
            for (index, argty) in argsty.iter().enumerate() {
                args[index] = *argty;
            }
        }
        let statements = body
            .iter()
            .map(|statement| self.resolve_stylesheet_statement(fileid, statement))
            .collect::<Result<Vec<_>>>()?;

        let usages = usages
            .iter()
            .map(|usage| self.resolve_style_usage(fileid, usage))
            .collect::<Result<Vec<_>>>()?;

        // Re-acquire write lock only for declaration creation and scope exit.
        let mut file = self.get_file_mut(fileid);
        file.create_declaration(HirDeclaration::new_stylesheet(
            args,
            statements,
            usages,
            span,
            DeclarationId::new(fileid, id.local_id),
            typeid,
        ));
        file.scopes.exit_scope();

        Ok(())
    }

    ///Resolves a style usage from the given `usage` expression. It's expected to be a function call. The reason is cause the same syntax for function call is used when calling styles
    pub(crate) fn resolve_style_usage(
        &self,
        fileid: FileId,
        usage: &ASTExpression,
    ) -> Result<HirStyleUsage> {
        let (name, args) = match &usage.kind {
            ASTExpressionKind::FunctionCall { name, args } => (name, args),
            _ => unreachable!("Style usage should be a function call on parsing"),
        };
        let symbol = self.intern_name(&name.identifier);
        let (decl, tyid) = self.find_declaration_by_name(&symbol, name.span)?;
        debug_assert!(matches!(*self.get_type(&tyid), HirType::Style { .. }));
        let params = args
            .iter()
            .map(|expr| self.generate_expression(fileid, expr, None))
            .collect::<Result<_>>()?;
        Ok(HirStyleUsage {
            style: decl,
            params,
            span: usage.span,
        })
    }
}
