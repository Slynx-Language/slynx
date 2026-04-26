use crate::hir::{
    PropertyId, Result, SlynxHir,
    error::{HIRError, HIRErrorKind},
    model::{
        ComponentMemberDeclaration, ComponentProperty, HirDeclaration, HirDeclarationKind, HirType,
    },
};
use common::ast::{
    ComponentMember, ComponentMemberKind, GenericIdentifier, ObjectField, Span, TypedName,
};

impl SlynxHir {
    ///Retrieves the type of something by knowing the provided `ref_ty` is a reference to it
    pub fn get_type_from_ref(&self, ref_ty: crate::hir::TypeId) -> &HirType {
        if let HirType::Reference { rf, .. } = self.types_module.get_type(&ref_ty) {
            self.types_module.get_type(rf)
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
        let mut fields = {
            let mut out = Vec::with_capacity(fields.len());
            for field in &fields {
                let symbol_name = self.symbols_module.intern(&name.identifier);
                if self.symbols_module.intern(&field.name.name) == symbol_name {
                    return Err(HIRError::recursive(symbol_name, field.name.span).into());
                }
                out.push(
                    self.retrieve_information_of_type(
                        &field.name.kind.identifier,
                        &field.name.span,
                    )?
                    .0,
                );
            }
            out
        };
        let identifier_symbol = self.modules.intern_name(&name.identifier);
        let (decl, declty) = if let Some(data) = self
            .declarations_module
            .retrieve_declaration_data_by_name(&identifier_symbol)
        {
            data
        } else {
            return Err(HIRError::name_unrecognized(identifier_symbol, name.span).into());
        };
        let HirType::Reference { rf, .. } = self.types_module.get_type(&declty) else {
            unreachable!("WTF, type of custom object should be a reference to its real type");
        };
        let rf = *rf;
        let HirType::Struct { fields: ty_field } = self.types_module.get_type_mut(&rf) else {
            unreachable!("WTF. Type of object should be a Struct ty");
        };

        ty_field.append(&mut fields);
        self.declarations.push(HirDeclaration {
            kind: HirDeclarationKind::Object,
            id: decl,
            ty: declty,
            span,
        });

        Ok(())
    }

    pub fn hoist_function(
        &mut self,
        name: &GenericIdentifier,
        args: &Vec<TypedName>,
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
                    Some(ty.map(|ty| ComponentProperty::new(modifier.clone(), name.clone(), ty)))
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

                    out.push(ComponentMemberDeclaration::Property {
                        id: PropertyId::new(),
                        index: prop_idx,
                        value: if let Some(rhs) = rhs {
                            Some(self.resolve_expr(rhs, Some(ty))?)
                        } else {
                            None
                        },
                        span: def.span,
                    });

                    self.create_variable(&name, ty, true);
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
