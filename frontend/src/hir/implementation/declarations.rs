use crate::hir::{
    Result, SlynxHir,
    error::HIRError,
    model::{ComponentMemberDeclaration, ComponentProperty, HirDeclaration, HirStatement, HirType},
};
use common::{
    ASTStatement, ASTStatementKind,
    ast::{ComponentMember, ComponentMemberKind, GenericIdentifier, ObjectField, Span, TypedName},
};

impl SlynxHir {
    ///Retrieves the type of something by knowing the provided `ref_ty` is a reference to it
    pub fn get_type_from_ref(&self, ref_ty: crate::hir::TypeId) -> &HirType {
        if let HirType::Reference { rf, .. } = self.get_type(&ref_ty) {
            self.get_type(rf)
        } else {
            unreachable!("The provided ref_ty should be of type Reference");
        }
    }

    pub fn resolve_object(
        &mut self,
        name: GenericIdentifier,
        fields: Vec<ObjectField>,
        span: Span,
    ) -> Result<()> {
        let mut fields = fields
            .into_iter()
            .map(|field| {
                let symbol_name = self.modules.intern_name(&name.identifier);
                if self.modules.intern_name(&field.name.name) == symbol_name {
                    Err(HIRError::recursive(symbol_name, field.name.span))
                } else {
                    self.retrieve_information_of_type(&field.name.kind.identifier, &field.name.span)
                        .map(|v| v.0)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let identifier_symbol = self.modules.intern_name(&name.identifier);
        let Some((decl, declty)) = self.modules.get_declaration_by_name(&identifier_symbol) else {
            return Err(HIRError::name_unrecognized(identifier_symbol, name.span));
        };
        let HirType::Reference { rf, .. } = self.get_type(&declty) else {
            unreachable!("WTF, type of custom object should be a reference to its real type");
        };
        let rf = *rf;
        let HirType::Struct { fields: ty_field } = self.get_type_mut(&rf) else {
            unreachable!("WTF. Type of object should be a Struct ty");
        };

        ty_field.append(&mut fields);
        self.declarations
            .push(HirDeclaration::new_object(decl, declty, span));

        Ok(())
    }

    pub fn hoist_function(
        &mut self,
        name: &GenericIdentifier,
        args: &[TypedName],
        return_type: &GenericIdentifier,
    ) -> Result<()> {
        let args = args
            .iter()
            .map(|arg| self.get_typeid_of_generic(&arg.kind))
            .collect::<Result<Vec<_>>>()?;
        let return_type = self.get_typeid_of_generic(return_type)?;
        self.modules
            .create_declaration(&name.identifier, HirType::new_function(args, return_type));

        Ok(())
    }

    pub fn resolve_function(
        &mut self,
        name: &GenericIdentifier,
        args: &[TypedName],
        body: Vec<ASTStatement>,
        span: &Span,
    ) -> Result<()> {
        let symbol = self.modules.intern_name(&name.identifier);
        let Some((decl, tyid)) = self.modules.get_declaration_by_name(&symbol) else {
            return Err(HIRError::name_unrecognized(symbol, name.span));
        };

        self.modules.enter_scope();

        let args = args
            .iter()
            .map(|arg| {
                let symbol = self.modules.intern_name(&arg.name);
                match self.retrieve_information_of_type(&arg.kind.identifier, &arg.kind.span) {
                    Ok((ty, _)) => self.create_variable(symbol, ty, &arg.span),
                    Err(e) => Err(e),
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let body_len = body.len();
        let statements = body
            .into_iter()
            .enumerate()
            .map(|(index, statement)| {
                let is_last = index + 1 == body_len;
                match statement {
                    // The last expression in a function body becomes the implicit return.
                    common::ast::ASTStatement {
                        kind: ASTStatementKind::Expression(expr),
                        ..
                    } if is_last => self.resolve_expr(expr, None).map(HirStatement::new_return),
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

    pub fn hoist_component(
        &mut self,
        name: &GenericIdentifier,
        members: &[ComponentMember],
    ) -> Result<()> {
        let props = members
            .iter()
            .filter_map(|member| match &member.kind {
                ComponentMemberKind::Property {
                    name, modifier, ty, ..
                } => {
                    let ty = match ty {
                        Some(generic) => self.get_typeid_of_name(&generic.identifier, &member.span),
                        _ => Ok(self.infer_type()),
                    };
                    Some(ty.map(|ty| ComponentProperty::new(*modifier, name.clone(), ty)))
                }
                ComponentMemberKind::Child(_) => None,
            })
            .collect::<Result<Vec<_>>>()?;

        self.modules
            .create_declaration(&name.identifier, HirType::new_component(props));
        Ok(())
    }

    pub fn resolve_component_defs(
        &mut self,
        def: Vec<ComponentMember>,
    ) -> Result<Vec<ComponentMemberDeclaration>> {
        let mut out = Vec::with_capacity(def.len());
        let mut prop_idx = 0;
        for def in def {
            match def.kind {
                ComponentMemberKind::Property { ty, rhs, name, .. } => {
                    let ty = if let Some(ty) = ty {
                        self.retrieve_information_of_type(&ty.identifier, &ty.span)?
                            .0
                    } else {
                        self.infer_type()
                    };
                    let rhs = if let Some(rhs) = rhs {
                        Some(self.resolve_expr(rhs, Some(ty))?)
                    } else {
                        None
                    };
                    out.push(ComponentMemberDeclaration::new_property(
                        prop_idx, rhs, def.span,
                    ));
                    let name = self.modules.intern_name(&name);

                    self.create_variable(name, ty, &def.span)?;
                    prop_idx += 1;
                }
                ComponentMemberKind::Child(child) => {
                    let component = self.resolve_component(child)?;
                    out.push(component);
                }
            }
        }
        Ok(out)
    }
}
