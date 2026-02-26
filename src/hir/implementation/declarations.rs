use color_eyre::eyre::Result;

use crate::hir::{
    PropertyId, SlynxHir,
    definitions::{
        ComponentMemberDeclaration, HirDeclaration, HirDeclarationKind, SpecializedComponent,
    },
    error::{HIRError, HIRErrorKind},
    types::HirType,
};
use crate::parser::ast::{
    ComponentMember, ComponentMemberKind, ComponentMemberValue, GenericIdentifier, ObjectField,
    Span, TypedName,
};

impl SlynxHir {
    pub fn resolve_object(
        &mut self,
        name: GenericIdentifier,
        fields: Vec<ObjectField>,
        span: Span,
    ) -> Result<()> {
        let mut fields = {
            let mut out = Vec::with_capacity(fields.len());
            for field in &fields {
                if self.symbols_module.intern(&field.name.name)
                    == self.symbols_module.intern(&name.identifier)
                {
                    return Err(HIRError {
                        kind: HIRErrorKind::RecursiveType {
                            ty: name.to_string(),
                        },
                        span: field.name.span.clone(),
                    }
                    .into());
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
        let symbol = self.get_symbol_of(&name.identifier, &name.span)?;
        let (decl, declty) = if let Some(data) = self
            .declarations_module
            .retrieve_declaration_data_by_name(&symbol)
        {
            data
        } else {
            return Err(HIRError {
                kind: HIRErrorKind::NameNotRecognized(name.identifier),
                span: name.span,
            }
            .into());
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

    pub fn hoist_object(
        &mut self,
        name: &GenericIdentifier,
        obj_fields: &[ObjectField],
    ) -> Result<()> {
        let name = self.symbols_module.intern(&name.identifier);
        let def_fields = obj_fields
            .iter()
            .map(|f| self.symbols_module.intern(&f.name.name))
            .collect();

        let ty = self
            .types_module
            .insert_unnamed_type(HirType::Struct { fields: Vec::new() });
        let ty = self.define_type(
            name,
            HirType::Reference {
                rf: ty,
                generics: Vec::new(),
            },
        );

        self.declarations_module.create_object(name, ty, def_fields);
        Ok(())
    }

    pub fn hoist_function(
        &mut self,
        name: &GenericIdentifier,
        args: &Vec<TypedName>,
        return_type: &GenericIdentifier,
    ) -> Result<()> {
        let args = {
            let mut vec = Vec::with_capacity(args.len());
            for arg in args {
                let (id, _) = self.retrieve_information_of_type(&arg.kind.identifier, &arg.span)?;
                vec.push(id);
            }
            vec
        };
        let func_ty = HirType::Function {
            args,
            return_type: self.get_typeid_of_name(&return_type.identifier, &return_type.span)?,
        };
        let symbol = self.symbols_module.intern(&name.identifier);
        let id = self.define_type(symbol, func_ty);
        self.create_declaration(&name.identifier, id);
        Ok(())
    }

    pub fn specialize_text(
        &mut self,
        values: Vec<ComponentMemberValue>,
        span: &Span,
    ) -> Result<SpecializedComponent> {
        let mut text = None;
        for value in values {
            match value {
                ComponentMemberValue::Assign {
                    prop_name,
                    rhs,
                    span,
                } => match prop_name.as_str() {
                    "text" => text = Some(self.resolve_expr(rhs, None)?),
                    _ => {
                        return Err(HIRError {
                            kind: HIRErrorKind::TypeNotRecognized(prop_name),
                            span: span.clone(),
                        }
                        .into());
                    }
                },
                ComponentMemberValue::Child(e) => {
                    return Err(HIRError {
                        kind: HIRErrorKind::InvalidChild { child: Box::new(e) },
                        span: span.clone(),
                    }
                    .into());
                }
            }
        }
        if let Some(text) = text {
            Ok(SpecializedComponent::Text {
                text: Box::new(text),
            })
        } else {
            Err(HIRError {
                kind: HIRErrorKind::MissingProperty {
                    prop_names: vec![String::from("text")],
                },
                span: span.clone(),
            }
            .into())
        }
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
                        self.types_module.infer_id()
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
                    match (&child.name.generic, child.name.identifier.as_str()) {
                        (None, "Text") => {
                            let text = self.specialize_text(child.values, &child.span)?;
                            out.push(ComponentMemberDeclaration::Specialized(text));
                        }
                        _ => {
                            let (id, _) = self.retrieve_information_of_type(
                                &child.name.identifier,
                                &child.span,
                            )?;
                            let values = self.resolve_component_members(child.values, id)?;
                            out.push(ComponentMemberDeclaration::Child {
                                name: id,
                                values,
                                span: child.span,
                            })
                        }
                    }
                }
            }
        }
        Ok(out)
    }
}
