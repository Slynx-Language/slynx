use crate::{
    Result, SlynxHir,
    error::HIRError,
    model::{
        ComponentMemberDeclaration, ComponentProperty, HirDeclaration, HirDeclarationKind,
        HirStatement, HirStyleUsage, HirType,
    },
    modules::DeclarationInfo,
};
use common::Span;
use slynx_parser::{
    ASTExpression, ASTExpressionKind, ASTStatement, ASTStatementKind, ComponentMember,
    ComponentMemberKind, GenericIdentifier, ObjectField, StyleSheetStatement, TypedName,
};

impl SlynxHir {
    pub(crate) fn hoist_enum(&mut self, name: &str, variants: &[String]) -> DeclarationInfo {
        let variants = variants
            .iter()
            .map(|variant| self.intern_name(variant))
            .collect();
        self.modules.create_enum(name, variants)
    }
    ///Hoists a `stylesheet` declaration
    pub(crate) fn hoist_stylesheet(&mut self, name: &str, args: &[TypedName]) {
        self.modules.create_declaration(
            name,
            HirType::Style {
                args: args.iter().map(|_| self.void_type()).collect(),
            },
        );
    }

    ///Resolves a `stylesheet` declaration
    pub(crate) fn resolve_stylesheet(
        &mut self,
        name: &GenericIdentifier,
        args: &[TypedName],
        usages: &[ASTExpression],
        body: &[StyleSheetStatement],
        span: Span,
    ) -> Result<()> {
        let symbol = self.modules.intern_name(&name.identifier);
        let Some((id, typeid)) = self.modules.get_declaration_by_name(&symbol) else {
            return Err(HIRError::name_unrecognized(symbol, name.span));
        };
        self.modules.enter_scope();

        let (args, argsty) = args
            .iter()
            .map(|arg| {
                let symbol = self.modules.intern_name(&arg.name);
                let ty_symbol = self.modules.intern_name(&arg.kind.identifier);
                let ty = self.get_type_of_name(ty_symbol, &arg.kind.span)?;
                self.create_variable(symbol, ty, &arg.span).map(|v| (v, ty))
            })
            .collect::<Result<(Vec<_>, Vec<_>)>>()?;
        {
            let HirType::Style { args } = self.get_type_mut(&typeid) else {
                unreachable!("Type of stylesheet should be style");
            };
            for (index, argty) in argsty.iter().enumerate() {
                args[index] = *argty;
            }
        }
        let statements = body
            .iter()
            .map(|statement| self.resolve_stylesheet_statement(statement))
            .collect::<Result<Vec<_>>>()?;

        let usages = usages
            .iter()
            .map(|usage| self.resolve_style_usage(usage))
            .collect::<Result<Vec<_>>>()?;

        self.declarations.push(HirDeclaration::new_stylesheet(
            args, statements, usages, span, id, typeid,
        ));
        self.modules.exit_scope();
        Ok(())
    }

    ///Resolves a style usage from the given `usage` expression. It's expected to be a function call. The reason is cause the same syntax for function call is used when calling styles
    pub(crate) fn resolve_style_usage(&mut self, usage: &ASTExpression) -> Result<HirStyleUsage> {
        let (name, args) = match &usage.kind {
            ASTExpressionKind::FunctionCall { name, args } => (name, args),
            _ => unreachable!("Style usage should be a function call on parsing"),
        };
        let symbol = self.modules.intern_name(&name.identifier);
        let Some((decl, tyid)) = self.modules.get_declaration_by_name(&symbol) else {
            return Err(HIRError::name_unrecognized(symbol, name.span));
        };
        debug_assert!(matches!(self.get_type(&tyid), HirType::Style { .. }));
        let params = args
            .iter()
            .map(|expr| self.generate_expression(expr, None))
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
        span: Span,
    ) -> Result<()> {
        let mut fields = fields
            .iter()
            .map(|field| {
                let symbol_name = self.modules.intern_name(&name.identifier);
                if self.modules.intern_name(&field.name.name) == symbol_name {
                    Err(HIRError::recursive(symbol_name, field.name.span))
                } else {
                    let name = self.intern_name(&field.name.kind.identifier);
                    self.get_type_of_name(name, &field.name.span)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let identifier_symbol = self.modules.intern_name(&name.identifier);
        let Some((decl, declty)) = self.modules.get_declaration_by_name(&identifier_symbol) else {
            return Err(HIRError::name_unrecognized(identifier_symbol, name.span));
        };
        let HirType::Reference { rf, .. } = self.get_type(&declty) else {
            unreachable!("Type of custom object should be a reference to its real type");
        };
        let rf = *rf;
        let HirType::Struct { fields: ty_field } = self.get_type_mut(&rf) else {
            unreachable!("Type of object should be a Struct ty");
        };

        ty_field.append(&mut fields);
        self.declarations
            .push(HirDeclaration::new_object(decl, declty, span));

        Ok(())
    }

    /// Hoists a function declaration by registering its signature without processing its body.
    pub(crate) fn hoist_function(
        &mut self,
        name: &GenericIdentifier,
        args: &[TypedName],
    ) -> Result<()> {
        let args = args.iter().map(|_| self.int32_type()).collect();
        let return_type = self.int32_type();
        self.modules
            .create_declaration(&name.identifier, HirType::new_function(args, return_type));

        Ok(())
    }

    /// Resolves a function declaration, type-checking its body and pushing the HIR declaration.
    pub(crate) fn resolve_function(
        &mut self,
        name: &GenericIdentifier,
        args: &[TypedName],
        return_type: &GenericIdentifier,
        body: &[ASTStatement],
        span: &Span,
    ) -> Result<()> {
        let symbol = self.modules.intern_name(&name.identifier);
        let Some((decl, tyid)) = self.modules.get_declaration_by_name(&symbol) else {
            return Err(HIRError::name_unrecognized(symbol, name.span));
        };

        self.modules.enter_scope();

        let (args, argsty) = args
            .iter()
            .map(|arg| {
                let ty_symbol = self.modules.intern_name(&arg.kind.identifier);
                let symbol = self.modules.intern_name(&arg.name);
                let ty = self.get_type_of_name(ty_symbol, &arg.kind.span)?;
                self.create_variable(symbol, ty, &arg.span).map(|v| (v, ty))
            })
            .collect::<Result<(Vec<_>, Vec<_>)>>()?;
        {
            let return_symbol = self.intern_name(&return_type.identifier);
            let ret_tyid = self.get_type_of_name(return_symbol, span)?;
            let HirType::Function {
                args,
                return_type: ret,
            } = self.get_type_mut(&tyid)
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
                        .generate_expression(expr, None)
                        .map(HirStatement::new_return),
                    statement => self.resolve_statement(statement),
                }
            })
            .collect::<Result<Vec<_>>>()?;

        self.declarations.push(HirDeclaration::new_function(
            statements, args, symbol, *span, decl, tyid,
        ));
        self.modules.exit_scope();
        Ok(())
    }

    /// Hoists a component declaration by registering its property layout without resolving children.
    pub(crate) fn hoist_component(
        &mut self,
        name: &GenericIdentifier,
        members: &[ComponentMember],
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

        self.modules
            .create_declaration(&name.identifier, HirType::new_component(props));
        Ok(())
    }

    /// Resolves the member definitions of a component body into [`ComponentMemberDeclaration`]s.
    pub(crate) fn resolve_component_defs(
        &mut self,
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
                        Some(self.generate_expression(rhs, Some(ty))?)
                    } else {
                        None
                    };
                    out.push(ComponentMemberDeclaration::new_property(
                        prop_idx, rhs, def.span,
                    ));
                    let name = self.modules.intern_name(name);

                    self.create_variable(name, ty, &def.span)?;
                    prop_idx += 1;
                }
                ComponentMemberKind::Child(child) => {
                    let component = self.resolve_component_expression(child)?;
                    out.push(ComponentMemberDeclaration::Child(component));
                }
            }
        }
        Ok(out)
    }

    ///Resolves a component declaration that contains the given `members` and the given `name`
    pub(crate) fn resolve_component_declaration(
        &mut self,
        members: &[ComponentMember],
        name: &GenericIdentifier,
        span: Span,
    ) -> Result<()> {
        self.modules.enter_scope();
        let symbol = self.modules.intern_name(&name.identifier);
        let Some((decl, ty)) = self.modules.get_declaration_by_name(&symbol) else {
            return Err(HIRError::name_unrecognized(symbol, span));
        };

        let defs = self.resolve_component_defs(members)?;
        self.declarations.push(HirDeclaration {
            id: decl,
            kind: HirDeclarationKind::ComponentDeclaration {
                props: defs,
                name: symbol,
            },
            ty,
            span,
        });
        self.modules.exit_scope();
        Ok(())
    }

    ///Resolves an alias type, mapping the given `name` to the given `target`
    pub(crate) fn resolve_alias(
        &mut self,
        name: &GenericIdentifier,
        target: &GenericIdentifier,
        span: Span,
    ) -> Result<()> {
        let alias_name = self.intern_name(&name.identifier);
        let intern_name = self.intern_name(&target.identifier);
        let target_ty = self.get_type_of_name(intern_name, &target.span)?;

        let Some(alias_ty) = self
            .modules
            .types_module
            .get_type_from_name_mut(&alias_name)
        else {
            return Err(HIRError::name_unrecognized(alias_name, name.span));
        };
        *alias_ty = HirType::new_ref(target_ty);
        let Some((decl, ty)) = self.modules.get_declaration_by_name(&alias_name) else {
            return Err(HIRError::name_unrecognized(intern_name, name.span));
        };
        self.declarations
            .push(HirDeclaration::new_alias(decl, ty, span));
        Ok(())
    }
}
