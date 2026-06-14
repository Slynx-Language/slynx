use crate::{
    DeclarationId, Result, SlynxHir,
    error::HIRError,
    model::{HirDeclaration, HirStatement, HirType},
    module_loader::FileId,
};
use common::{Span, VisibilityModifier};
use slynx_parser::{ASTStatement, ASTStatementKind, GenericIdentifier, TypedName};

impl SlynxHir {
    /// Hoists a function declaration by registering its signature without processing its body.
    pub(crate) fn hoist_function(
        &self,
        file: FileId,
        name: &GenericIdentifier,
        args: &[TypedName],
        visibility: VisibilityModifier,
    ) -> Result<DeclarationId> {
        let args = args.iter().map(|_| self.int32_type()).collect();
        let return_type = self.int32_type();
        let symbol = self.intern_name(&name.identifier);
        let ty = self
            .types_module
            .create_type(symbol, HirType::new_function(args, return_type));
        let local = self
            .get_file_mut(file)
            .declarations
            .register_declaration_metadata(symbol, ty, visibility);

        Ok(DeclarationId::new(file, local))
    }

    /// Resolves a function declaration, type-checking its body and pushing the HIR declaration.
    pub(crate) fn resolve_function(
        &self,
        fileid: FileId,
        name: &GenericIdentifier,
        args: &[TypedName],
        return_type: &GenericIdentifier,
        body: &[ASTStatement],
        span: &Span,
    ) -> Result<()> {
        let symbol = self.intern_name(&name.identifier);
        let (decl, tyid) = self.find_declaration_by_name(&symbol, name.span)?;

        // Hold write lock only for scope entry, release before subcalls that need read locks.
        self.get_file_mut(fileid).scopes.enter_scope();

        let (args, argsty) = args
            .iter()
            .map(|arg| {
                let ty_symbol = self.intern_name(&arg.kind.identifier);
                let symbol = self.intern_name(&arg.name);
                let ty = self.get_type_of_name(ty_symbol, &arg.kind.span)?;
                self.create_variable(fileid, symbol, ty, &arg.span)
                    .map(|v| (v, ty))
            })
            .collect::<Result<(Vec<_>, Vec<_>)>>()?;
        {
            let return_symbol = self.intern_name(&return_type.identifier);
            let ret_tyid = self.get_type_of_name(return_symbol, span)?;
            let mut guard = self.get_type_mut(tyid);
            let HirType::Function {
                args,
                return_type: ret,
            } = &mut *guard
            else {
                unreachable!("Type of function should be function");
            };
            for (index, argty) in argsty.iter().enumerate() {
                args[index] = *argty;
            }
            *ret = ret_tyid;
        }
        let statements = body
            .iter()
            .enumerate()
            .map(|(index, statement)| {
                let is_last = index + 1 == body.len();
                match statement {
                    ASTStatement {
                        kind: ASTStatementKind::Expression(expr),
                        ..
                    } if is_last => self
                        .generate_expression(fileid, expr, None)
                        .map(HirStatement::new_return),
                    statement => self.resolve_statement(fileid, statement),
                }
            })
            .collect::<Result<Vec<_>>>()?;

        // Re-acquire write lock only for declaration creation and scope exit.
        let mut file = self.get_file_mut(fileid);
        file.create_declaration(HirDeclaration::new_function(
            statements, args, symbol, *span, decl, tyid,
        ));
        file.scopes.exit_scope();
        Ok(())
    }

    ///Resolves an alias type, mapping the given `name` to the given `target`
    pub(crate) fn resolve_alias(
        &self,
        name: &GenericIdentifier,
        target: &GenericIdentifier,
    ) -> Result<()> {
        let alias_name = self.intern_name(&name.identifier);
        let intern_name = self.intern_name(&target.identifier);
        let target_ty = self.get_type_of_name(intern_name, &target.span)?;
        {
            let Some(mut alias_ty) = self.types_module.get_type_from_name_mut(&alias_name) else {
                return Err(HIRError::name_unrecognized(alias_name, name.span));
            };
            *alias_ty = HirType::new_ref(target_ty);
        }
        Ok(())
    }
}
