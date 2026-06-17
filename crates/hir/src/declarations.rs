use crate::{
    DeclarationId, Result, SlynxHir, TypeId,
    error::HIRError,
    model::{HirDeclaration, HirStatement, HirType},
    module_loader::FileId,
};
use common::{Span, VisibilityModifier};
use slynx_parser::{ASTStatement, ASTStatementKind, GenericIdentifier, TypedName};

pub struct FunctionData<'a> {
    pub(crate) name: &'a GenericIdentifier,
    pub(crate) args: &'a [TypedName],
    pub(crate) return_type: &'a GenericIdentifier,
    pub(crate) body: &'a [ASTStatement],
    pub(crate) span: &'a Span,
    pub(crate) external: bool,
}

impl SlynxHir {
    pub(crate) fn create_static(&self, file: FileId, name: &str) -> Result<DeclarationId> {
        let symbol = self.intern_name(name);
        let ty = self.types_module.create_type(symbol, HirType::Infer);
        let local = self
            .get_file_mut(file)
            .declarations
            .register_declaration_metadata(symbol, ty, VisibilityModifier::Private);
        Ok(DeclarationId::new(file, local))
    }
    pub(crate) fn resolve_static_type(
        &self,
        name: &str,
        ty: &GenericIdentifier,
        span: &Span,
    ) -> Result<()> {
        let ty_symbol = self.intern_name(&ty.identifier);
        let symbol = self.intern_name(name);
        let (_, ty) = self.find_declaration_by_name(&symbol, *span)?;
        {
            let mut ty = self.types_module.get_type_mut(ty);
            *ty = HirType::new_ref(self.get_type_of_name(ty_symbol, span)?);
        };
        Ok(())
    }

    pub(crate) fn create_static_declaration(
        &self,
        name: &str,
        span: &Span,
        external: bool,
    ) -> Result<()> {
        let symbol = self.intern_name(name);
        let (id, ty) = self.find_declaration_by_name(&symbol, *span)?;
        let tyid = self.get_file(id.file_id).get_declaration_type(id.local_id);
        self.get_file_mut(id.file_id)
            .create_declaration(HirDeclaration::new_static(id, tyid, *span, external));
        Ok(())
    }

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
        FunctionData {
            name,
            args,
            return_type,
            body,
            span,
            external,
        }: FunctionData,
        self_type: Option<TypeId>,
    ) -> Result<()> {
        let symbol = self.intern_name(&name.identifier);
        let mangled_symbol = if let Some(self_type) = self_type {
            self.get_name_of_type(self_type)
                .map(|type_name| {
                    let type_name = self.get_name(type_name);
                    self.intern_name(&format!("{}_{}", name.identifier, type_name))
                })
                .unwrap_or(symbol)
        } else {
            symbol
        };
        let (decl, tyid) = self.find_declaration_by_name(&symbol, name.span)?;

        // Hold write lock only for scope entry, release before subcalls that need read locks.
        self.get_file_mut(fileid).scopes.enter_scope();

        let (args, argsty) = args
            .iter()
            .map(|arg| {
                let ty_symbol = self.intern_name(&arg.kind.identifier);
                let symbol = self.intern_name(&arg.name);
                let ty = if (arg.kind.identifier == "Self" || arg.kind.identifier == "self")
                    && let Some(self_type) = self_type
                {
                    self_type
                } else {
                    self.get_type_of_name(ty_symbol, &arg.kind.span)?
                };
                self.create_variable(fileid, symbol, ty, &arg.span)
                    .map(|v| (v, ty))
            })
            .collect::<Result<(Vec<_>, Vec<_>)>>()?;
        {
            let return_symbol = self.intern_name(&return_type.identifier);
            let ret_tyid = if return_type.identifier == "Self"
                && let Some(self_type) = self_type
            {
                self_type
            } else {
                self.get_type_of_name(return_symbol, span)?
            };
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
            statements, args, mangled_symbol, *span, decl, tyid, external,
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
        // Propagate externality through aliases
        if self.types_module.is_external(&target_ty)
            && let Some(alias_id) = self.types_module.get_id(&alias_name)
        {
            self.types_module.mark_external(alias_id);
        }
        Ok(())
    }
}
