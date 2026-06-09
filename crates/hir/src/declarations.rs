use crate::{
    DeclarationId, Result, SlynxHir,
    error::HIRError,
    model::{
        ComponentMemberDeclaration, ComponentProperty, HirDeclaration, HirDeclarationKind,
        HirStatement, HirStyleUsage, HirType,
    },
    module_loader::FileId,
};
use common::{Span, VisibilityModifier};
use slynx_parser::{
    ASTExpression, ASTExpressionKind, ASTStatement, ASTStatementKind, ComponentMember,
    ComponentMemberKind, GenericIdentifier, ObjectField, StyleSheetStatement, TypedName,
};

impl SlynxHir {
    pub(crate) fn create_empty_object(
        &mut self,
        file: FileId,
        name: &GenericIdentifier,
        fields: &[ObjectField],
        visibility: VisibilityModifier,
    ) {
        let name = self.intern_name(&name.identifier);
        let def_fields = fields
            .iter()
            .map(|f| self.intern_name(&f.name.name))
            .collect();
        let struct_ty = self
            .types_module
            .create_unnamed_type(HirType::new_struct(Vec::new()));
        let ty = self
            .types_module
            .create_type(name, HirType::new_ref(struct_ty));
        self.get_file_mut(file)
            .declarations
            .register_object(name, ty, Vec::new(), visibility);
        self.types_module.objects.insert(ty, def_fields);
    }
    ///Hoists a `stylesheet` declaration
    pub(crate) fn hoist_stylesheet(
        &mut self,
        file: FileId,
        name: &str,
        args: &[TypedName],
        visibility: VisibilityModifier,
    ) {
        let name = self.intern_name(name);
        let ty = self.types_module.create_type(
            name,
            HirType::Style {
                args: args.iter().map(|_| self.void_type()).collect(),
            },
        );
        self.get_file_mut(file)
            .declarations
            .register_declaration_metadata(name, ty, visibility);
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
        self.get_file(fileid).scopes.enter_scope();

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
        let file = self.get_file_mut(fileid);
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
        debug_assert!(matches!(self.get_type(&tyid), HirType::Style { .. }));
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

    /// Resolves an object declaration, filling in its field types and pushing the declaration.
    pub(crate) fn resolve_object(
        &mut self,
        name: &GenericIdentifier,
        fields: &[ObjectField],
    ) -> Result<()> {
        let mut fields = fields
            .iter()
            .map(|field| {
                let symbol_name = self.intern_name(&name.identifier);
                if self.intern_name(&field.name.name) == symbol_name {
                    Err(HIRError::recursive(symbol_name, field.name.span))
                } else {
                    let name = self.intern_name(&field.name.kind.identifier);
                    self.get_type_of_name(name, &field.name.span)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let identifier_symbol = self.intern_name(&name.identifier);
        let (_, declty) = self.find_declaration_by_name(&identifier_symbol, name.span)?;

        let HirType::Reference { rf, .. } = self.get_type(&declty) else {
            unreachable!("Type of custom object should be a reference to its real type");
        };
        let rf = *rf;
        let HirType::Struct { fields: ty_field } = self.get_type_mut(rf) else {
            unreachable!("Type of object should be a Struct ty");
        };

        ty_field.append(&mut fields);

        Ok(())
    }

    /// Hoists a function declaration by registering its signature without processing its body.
    pub(crate) fn hoist_function(
        &mut self,
        file: FileId,
        name: &GenericIdentifier,
        args: &[TypedName],
        visibility: VisibilityModifier,
    ) -> Result<()> {
        let args = args.iter().map(|_| self.int32_type()).collect();
        let return_type = self.int32_type();
        let symbol = self.intern_name(&name.identifier);
        let ty = self
            .types_module
            .create_type(symbol, HirType::new_function(args, return_type));
        self.get_file_mut(file)
            .declarations
            .register_declaration_metadata(symbol, ty, visibility);

        Ok(())
    }

    /// Resolves a function declaration, type-checking its body and pushing the HIR declaration.
    pub(crate) fn resolve_function(
        &mut self,
        fileid: FileId,
        name: &GenericIdentifier,
        args: &[TypedName],
        return_type: &GenericIdentifier,
        body: &[ASTStatement],
        span: &Span,
    ) -> Result<()> {
        let symbol = self.intern_name(&name.identifier);
        let (decl, tyid) = self.find_declaration_by_name(&symbol, name.span)?;
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
            let HirType::Function {
                args,
                return_type: ret,
            } = self.get_type_mut(tyid)
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
                    // The last expression in a function body becomes the implicit return.
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
        let file = self.get_file_mut(fileid);
        file.create_declaration(HirDeclaration::new_function(
            statements, args, symbol, *span, decl, tyid,
        ));
        file.scopes.exit_scope();
        Ok(())
    }

    /// Hoists a component declaration by registering its property layout without resolving children.
    pub(crate) fn hoist_component(
        &mut self,
        file: FileId,
        name: &GenericIdentifier,
        members: &[ComponentMember],
        visibility: VisibilityModifier,
    ) -> Result<()> {
        let props = members
            .iter()
            .filter_map(|member| match &member.kind {
                ComponentMemberKind::Property {
                    name,
                    modifier,
                    ty: Some(generic),
                    ..
                } => {
                    let name = self.intern_name(name);
                    let ty_name = self.intern_name(&generic.identifier);
                    let ty = match self.get_type_of_name(ty_name, &member.span) {
                        Ok(v) => v,
                        Err(e) => return Some(Err(e)),
                    };
                    Some(Ok(ComponentProperty::new(*modifier, name, ty)))
                }
                ComponentMemberKind::Property {
                    name,
                    modifier,
                    ty: None,
                    ..
                } => {
                    let name = self.intern_name(name);
                    Some(Ok(ComponentProperty::new(
                        *modifier,
                        name,
                        self.infer_type(),
                    )))
                }
                ComponentMemberKind::Child(_) => None,
            })
            .collect::<Result<Vec<_>>>()?;
        let symbol = self.intern_name(&name.identifier);
        let ty = self
            .types_module
            .create_type(symbol, HirType::new_component(props));
        self.get_file_mut(file)
            .declarations
            .register_declaration_metadata(symbol, ty, visibility);
        Ok(())
    }

    /// Resolves the member definitions of a component body into [`ComponentMemberDeclaration`]s.
    pub(crate) fn resolve_component_defs(
        &mut self,
        fileid: FileId,
        def: &[ComponentMember],
    ) -> Result<Vec<ComponentMemberDeclaration>> {
        let mut out = Vec::with_capacity(def.len());
        let mut prop_idx = 0;
        for def in def {
            match &def.kind {
                ComponentMemberKind::Property { ty, rhs, name, .. } => {
                    let ty = if let Some(ty) = ty {
                        let symbol = self.intern_name(&ty.identifier);
                        self.get_type_of_name(symbol, &ty.span)?
                    } else {
                        self.infer_type()
                    };
                    let rhs = if let Some(rhs) = rhs {
                        Some(self.generate_expression(fileid, rhs, Some(ty))?)
                    } else {
                        None
                    };
                    out.push(ComponentMemberDeclaration::new_property(
                        prop_idx, rhs, def.span,
                    ));
                    let name = self.intern_name(name);

                    self.create_variable(fileid, name, ty, &def.span)?;
                    prop_idx += 1;
                }
                ComponentMemberKind::Child(child) => {
                    let component = self.resolve_component_expression(fileid, child)?;
                    out.push(ComponentMemberDeclaration::Child(component));
                }
            }
        }
        Ok(out)
    }

    ///Resolves a component declaration that contains the given `members` and the given `name`
    pub(crate) fn resolve_component_declaration(
        &mut self,
        fileid: FileId,
        members: &[ComponentMember],
        name: &GenericIdentifier,
        span: Span,
    ) -> Result<()> {
        let symbol = self.intern_name(&name.identifier);
        let defs = self.resolve_component_defs(fileid, members)?;

        let (decl, ty) = self.find_declaration_by_name(&symbol, span)?;

        let file = self.get_file_mut(fileid);
        file.scopes.enter_scope();
        file.create_declaration(HirDeclaration {
            id: decl,
            kind: HirDeclarationKind::ComponentDeclaration {
                props: defs,
                name: symbol,
            },
            ty,
            span,
            visibility: Default::default(),
        });
        file.scopes.exit_scope();
        Ok(())
    }

    ///Resolves an alias type, mapping the given `name` to the given `target`
    pub(crate) fn resolve_alias(
        &mut self,
        name: &GenericIdentifier,
        target: &GenericIdentifier,
    ) -> Result<()> {
        let alias_name = self.intern_name(&name.identifier);
        let intern_name = self.intern_name(&target.identifier);
        let target_ty = self.get_type_of_name(intern_name, &target.span)?;
        {
            let Some(alias_ty) = self.types_module.get_type_from_name_mut(&alias_name) else {
                return Err(HIRError::name_unrecognized(alias_name, name.span));
            };
            *alias_ty = HirType::new_ref(target_ty);
        }
        Ok(())
    }
}
