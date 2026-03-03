use color_eyre::eyre::Result;

use super::TypeChecker;

use crate::{
    checker::error::{TypeError, TypeErrorKind},
    hir::{
        TypeId,
        definitions::{
            ComponentMemberDeclaration, HirExpression, HirExpressionKind, HirStatement,
            HirStatementKind, SpecializedComponent,
        },
        types::{FieldMethod, HirType},
    },
};
impl TypeChecker {
    pub(super) fn resolve_specialized(
        &mut self,
        component: &mut SpecializedComponent,
    ) -> Result<TypeId> {
        match component {
            SpecializedComponent::Text { text } => {
                text.ty = self.unify(&text.ty, &self.types_module.str_id(), &text.span)?;
            }
            SpecializedComponent::Div { children } => {
                self.resolve_component_members(children, self.types_module.generic_component_id())?;
            }
        }
        Ok(self.types_module.generic_component_id())
    }

    pub(super) fn resolve_statments(
        &mut self,
        statments: &mut Vec<HirStatement>,
        ty: &TypeId,
    ) -> Result<()> {
        let HirType::Function { return_type, .. } = self.types_module.get_type(ty).clone() else {
            unreachable!();
        };
        for statment in statments {
            match &mut statment.kind {
                HirStatementKind::Variable { value, .. } => {
                    value.ty = self.get_type_of_expr(value)?;
                }
                HirStatementKind::Return { expr } => {
                    expr.ty = self.get_type_of_expr(expr)?;
                    expr.ty = self.unify(&expr.ty, &return_type, &statment.span)?;
                }
                HirStatementKind::Expression { expr } => {
                    expr.ty = self.get_type_of_expr(expr)?;
                }
                HirStatementKind::Assign { lhs, value } => {
                    let refty = match self.types_module.get_type(&lhs.ty) {
                        HirType::Field(FieldMethod::Type(_, _)) => lhs.ty,
                        HirType::Field(FieldMethod::Variable(v, name)) => {
                            let object_ty =
                                *self.types_module.get_variable(v).ok_or(TypeError {
                                    kind: TypeErrorKind::Unrecognized,
                                    span: lhs.span.clone(),
                                })?;
                            let HirType::Reference { rf, .. } =
                                self.retrieve_reference_of(v, &lhs.span)?
                            else {
                                unreachable!();
                            };
                            if let Some(index) = self
                                .structs
                                .get(&object_ty)
                                .expect("Type should be defined")
                                .iter()
                                .position(|f| f == name)
                            {
                                let HirExpressionKind::FieldAccess {
                                    ref mut field_index,
                                    ..
                                } = lhs.kind
                                else {
                                    unreachable!();
                                };
                                *field_index = index;
                                *self.types_module.get_type_mut(&lhs.ty) =
                                    HirType::Field(FieldMethod::Type(rf, index));
                                lhs.ty
                            } else {
                                return Err(TypeError {
                                    kind: TypeErrorKind::Unrecognized,
                                    span: lhs.span.clone(),
                                }
                                .into());
                            }
                        }
                        HirType::VarReference(_) => lhs.ty,
                        _ => unreachable!(),
                    };

                    let ty = self.resolve(&lhs.ty, &statment.span)?;
                    lhs.ty = refty;
                    value.ty = self.get_type_of_expr(value)?;
                    value.ty = self.unify(&ty, &value.ty, &value.span)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn resolve_object_types(
        &mut self,
        ty: &HirType,
        fields: &mut [HirExpression],
    ) -> Result<()> {
        let HirType::Struct { fields: fields_tys } = ty else {
            unreachable!("When resolving object types, a type 'struct' should be provided");
        };
        for (idx, f) in fields.iter_mut().enumerate() {
            f.ty = self.unify(&fields_tys[idx], &f.ty, &f.span)?;
        }
        Ok(())
    }

    pub fn get_type_from_ref(&self, ref_ty: TypeId) -> &HirType {
        if let HirType::Reference { rf, .. } = self.types_module.get_type(&ref_ty) {
            self.types_module.get_type(rf)
        } else {
            unreachable!("The provided ref_ty should be of type Reference");
        }
    }

    /// Retrieves the type of the provided `expr`. Returns infer if it could not be inferred.
    pub(super) fn get_type_of_expr(&mut self, expr: &mut HirExpression) -> Result<TypeId> {
        let expected = expr.ty;

        let calc = match expr.kind {
            HirExpressionKind::FunctionCall {
                name,
                args: ref f_args,
            } => {
                let t = self.declarations[name.as_raw() as usize];
                let HirType::Function { args, return_type } = self.types_module.get_type(&t) else {
                    unreachable!();
                };
                let return_type = *return_type;
                for (f_arg, t_args) in f_args.iter().zip(args.clone()) {
                    self.unify(&f_arg.ty, &t_args, &f_arg.span)?;
                }
                return_type
            }
            HirExpressionKind::Int(_) => self.types_module.int_id(),
            HirExpressionKind::Float(_) => self.types_module.float_id(),
            HirExpressionKind::StringLiteral(_) => self.types_module.str_id(),
            HirExpressionKind::Binary {
                ref mut lhs,
                ref mut rhs,
                ref op,
            } => {
                let lhs_ty = self.get_type_of_expr(lhs)?;
                let rhs_ty = self.get_type_of_expr(rhs)?;
                let out = self.unify(&lhs_ty, &rhs_ty, &expr.span)?;
                if op.is_logical() {
                    self.types_module.bool_id()
                } else {
                    out
                }
            }
            HirExpressionKind::Identifier(_) => self.resolve(&expr.ty, &expr.span)?,
            HirExpressionKind::Component {
                name,
                ref mut values,
            } => self.resolve_component_members(values, name)?,
            HirExpressionKind::Object {
                name,
                ref mut fields,
            } => {
                let obj = self.get_type_from_ref(name).clone();
                self.resolve_object_types(&obj, fields)?;
                name
            }

            HirExpressionKind::FieldAccess {
                ref mut field_index,
                expr: ref mut e,
            } => {
                self.get_type_of_expr(e)?;

                match self.types_module.get_type(&expr.ty) {
                    HirType::Field(FieldMethod::Variable(id, name)) => {
                        let object_ty = *self.types_module.get_variable(id).ok_or(TypeError {
                            kind: TypeErrorKind::Unrecognized,
                            span: expr.span.clone(),
                        })?;
                        let HirType::Reference { rf, .. } =
                            self.retrieve_reference_of(id, &expr.span)?
                        else {
                            unreachable!();
                        };
                        if let Some(index) = self
                            .structs
                            .get(&object_ty)
                            .unwrap()
                            .iter()
                            .position(|field| field == name)
                        {
                            *field_index = index;
                            let HirType::Struct { fields } = self.types_module.get_type(&rf) else {
                                unreachable!()
                            };
                            fields[index]
                        } else {
                            return Err(TypeError {
                                kind: TypeErrorKind::Unrecognized,
                                span: expr.span.clone(),
                            }
                            .into());
                        }
                    }
                    ref u => unimplemented!("{u:?}"),
                }
            }
            HirExpressionKind::Bool(_) => self.types_module.bool_id(),
            HirExpressionKind::Specialized(ref mut s) => self.resolve_specialized(s)?,
        };

        expr.ty = self.unify(&expected, &calc, &expr.span)?;
        Ok(expr.ty)
    }

    pub(super) fn default_expr(&mut self, expr: &mut HirExpression) -> Result<()> {
        match expr.kind {
            HirExpressionKind::FunctionCall {
                name,
                args: ref f_args,
            } => {
                let HirType::Function { args, return_type } = self
                    .types_module
                    .get_type(&self.declarations[name.as_raw() as usize])
                else {
                    unreachable!()
                };
                let return_type = *return_type;
                for (f_arg, t_args) in f_args.iter().zip(args.clone()) {
                    self.unify(&f_arg.ty, &t_args, &f_arg.span)?;
                }
                expr.ty = self.unify(&expr.ty, &return_type, &expr.span)?;
            }
            HirExpressionKind::Bool(_) => {
                expr.ty = self.unify(&expr.ty, &self.types_module.bool_id(), &expr.span)?
            }
            HirExpressionKind::StringLiteral(_) => {
                expr.ty = self.unify(&expr.ty, &self.types_module.str_id(), &expr.span)?
            }
            HirExpressionKind::Int(_) => {
                expr.ty = self.unify(&expr.ty, &self.types_module.int_id(), &expr.span)?
            }
            HirExpressionKind::Float(_) => {
                expr.ty = self.unify(&expr.ty, &self.types_module.float_id(), &expr.span)?
            }
            HirExpressionKind::Binary {
                ref mut lhs,
                ref mut rhs,
                op,
            } => {
                self.default_expr(rhs)?;
                self.default_expr(lhs)?;
                if op.is_logical() {
                    expr.ty = self.types_module.bool_id();
                } else {
                    expr.ty = self.unify(&rhs.ty, &lhs.ty, &expr.span)?;
                }
            }
            HirExpressionKind::Identifier(_) => {
                expr.ty = self.resolve(&expr.ty, &expr.span)?;
            }
            HirExpressionKind::Specialized(_) => {
                expr.ty = self.unify(
                    &expr.ty,
                    &self.types_module.generic_component_id(),
                    &expr.span,
                )?
            }
            HirExpressionKind::Object { .. } => {
                expr.ty = self.resolve(&expr.ty, &expr.span)?;
            }
            HirExpressionKind::FieldAccess {
                expr: ref mut parent,
                ..
            } => {
                parent.ty = self.resolve(&parent.ty, &expr.span)?;
                expr.ty = self.resolve(&expr.ty, &expr.span)?;
            }
            HirExpressionKind::Component {
                ref name,
                ref mut values,
            } => {
                let ty = self.types_module.insert_unnamed_type(HirType::Reference {
                    rf: *name,
                    generics: Vec::new(),
                });
                expr.ty = ty;
                for val in values {
                    match val {
                        ComponentMemberDeclaration::Property {
                            index, value, span, ..
                        } => {
                            if let Some(value) = value {
                                let ty = self.get_type_of_expr(value)?;
                                let resolved = self.resolve(&expr.ty, &expr.span)?;
                                let HirType::Component { props } =
                                    self.types_module.get_type(&resolved)
                                else {
                                    unreachable!(
                                        "Component expression should be of type component"
                                    );
                                };
                                let propty = props[*index].2;
                                let ty = self.unify(&propty, &ty, span)?;

                                let HirType::Component { props } =
                                    self.types_module.get_type_mut(&resolved)
                                else {
                                    unreachable!(
                                        "Component expression should be of type component"
                                    );
                                };
                                props[*index].2 = ty;
                            }
                        }
                        ComponentMemberDeclaration::Specialized(_) => {}
                        ComponentMemberDeclaration::Child { name, values, .. } => {
                            self.resolve_component_members(values, *name)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
