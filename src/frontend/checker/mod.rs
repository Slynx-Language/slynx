mod decl;
pub mod error;
mod expr;
mod statement;
use std::collections::HashMap;

use color_eyre::eyre::Result;

use crate::{
    frontend::checker::error::{IncompatibleComponentReason, TypeError, TypeErrorKind},
    frontend::parser::ast::Span,
    middleend::hir::{
        SlynxHir, TypeId, VariableId,
        definitions::{HirDeclaration, HirDeclarationKind},
        symbols::SymbolPointer,
        types::{FieldMethod, HirType, TypesModule},
    },
};

#[derive(Debug)]
pub struct TypeChecker {
    ///A an array of declaration types
    declarations: Vec<TypeId>,

    types_module: TypesModule,
    /// The type of everything that is expected to have some
    types: HashMap<TypeId, HirType>,
    structs: HashMap<TypeId, Vec<SymbolPointer>>,
}

impl TypeChecker {
    /// Checks the types of the provided `hir` and mutates them if needed. Any that could not be inferred but, yet is valid, is
    /// at the end, returned as it's default type. Returns the type module to be used on the next steps
    pub fn check(hir: &mut SlynxHir) -> Result<TypesModule> {
        let mut inner = Self {
            types: HashMap::new(),
            structs: std::mem::take(&mut hir.declarations_module.objects),
            types_module: std::mem::take(&mut hir.types_module),
            declarations: Vec::new(),
        };
        for decl in &mut hir.declarations {
            inner.check_decl(decl)?;
        }
        // for the ones that couldn't be inferred, put their default
        for decl in &mut hir.declarations {
            inner.set_default(decl)?;
        }

        Ok(inner.types_module)
    }

    fn substitute(&mut self, id: TypeId, ty: HirType) {
        self.types.insert(id, ty);
    }

    /// Returns a reference to a struct based on the provided `id` which is expected to be the id of a VarReference type.
    /// This returns a reference type to an object type.
    fn retrieve_reference_of(&self, id: &VariableId, span: &Span) -> Result<HirType, TypeError> {
        if let Some(v) = self.types_module.get_variable(id) {
            let ty = self.types_module.get_type(v);
            match ty {
                HirType::Reference { .. } => Ok(ty.clone()),
                HirType::VarReference(id) => self.retrieve_reference_of(id, span),
                _ => Err(TypeError {
                    kind: TypeErrorKind::NotARef(*id, ty.clone()),
                    span: span.clone(),
                }),
            }
        } else {
            unreachable!("Type should have been determined");
        }
    }

    /// Resolves recursively the names of the types. If A -> B, B -> int; then we assume that A -> int
    fn resolve(&mut self, ty: &TypeId, span: &Span) -> Result<TypeId> {
        let referedty = self.types_module.get_type(ty).clone();
        match referedty {
            HirType::Field(FieldMethod::Type(rf, index)) => {
                let ty = self.resolve(&rf, span)?;
                if let HirType::Struct { fields } = self.types_module.get_type(&ty) {
                    Ok(fields[index])
                } else {
                    Err(TypeError {
                        kind: TypeErrorKind::IncompatibleTypes {
                            expected: self.types_module.get_type(&ty).clone(),
                            received: HirType::Struct { fields: Vec::new() },
                        },
                        span: span.clone(),
                    }
                    .into())
                }
            }
            HirType::Field(FieldMethod::Variable(var_id, n)) => {
                let HirType::Reference { rf, .. } = self.retrieve_reference_of(&var_id, span)?
                else {
                    unreachable!();
                };
                let object_ty = *self.types_module.get_variable(&var_id).ok_or(TypeError {
                    kind: TypeErrorKind::Unrecognized,
                    span: span.clone(),
                })?;

                let concrete_type = self.types_module.get_type(&rf);
                let s_fields = self.structs.get(&object_ty).ok_or(TypeError {
                    kind: TypeErrorKind::Unrecognized,
                    span: span.clone(),
                })?;
                let HirType::Struct { fields } = concrete_type else {
                    unreachable!("Type should be a struct. Fields only happen to structs");
                };
                if let Some(index) = s_fields.iter().position(|name| *name == n) {
                    Ok(fields[index])
                } else {
                    Err(TypeError {
                        kind: TypeErrorKind::Unrecognized,
                        span: span.clone(),
                    }
                    .into())
                }
            }
            HirType::Reference { rf, .. } => Ok(rf),
            HirType::VarReference(rf) => {
                if let Some(ty) = self.types_module.get_variable(&rf).cloned() {
                    Ok(self.resolve(&ty, span)?)
                } else {
                    unreachable!("Variable type should be defined here");
                }
            }
            HirType::Component { props } => {
                let mut resolved_props = {
                    let mut tys = Vec::with_capacity(props.len());
                    for prop in props {
                        tys.push((prop.0.clone(), prop.1.clone(), self.resolve(&prop.2, span)?));
                    }
                    tys
                };
                let HirType::Component { props } = self.types_module.get_type_mut(ty) else {
                    unreachable!();
                };
                props.clear();
                props.append(&mut resolved_props);
                Ok(*ty)
            }

            _ => Ok(*ty),
        }
    }

    /// Tries to unify types `a` and `b` if possible
    fn unify(&mut self, a: &TypeId, b: &TypeId, span: &Span) -> Result<TypeId> {
        let a = self.resolve(a, span)?;
        let b = self.resolve(b, span)?;

        match (&a, &b) {
            (a, b) if a == b => Ok(*a),
            (out, inf) | (inf, out)
                if *out != self.types_module.infer_id() && *inf == self.types_module.infer_id() =>
            {
                Ok(*out)
            }
            (a, b) => {
                let concrete_a = self.types_module.get_type(a).clone();
                let concrete_b = self.types_module.get_type(b).clone();

                match (&concrete_a, &concrete_b) {
                    (_, HirType::Infer) => Ok(*a),
                    (HirType::Infer, _) => Ok(*b),
                    (HirType::Reference { rf, .. }, _) => self.unify_with_ref(*rf, *b, span),
                    (_, HirType::Reference { rf, .. }) => self.unify_with_ref(*rf, *b, span),
                    (
                        HirType::Component { props: aprops },
                        HirType::Component { props: bprops },
                    ) => {
                        if aprops.len() != bprops.len() {
                            return Err(TypeError {
                                kind: TypeErrorKind::IncompatibleComponent {
                                    reason: IncompatibleComponentReason::DifferentPropAmount {
                                        rhs: aprops.len(),
                                        lhs: bprops.len(),
                                    },
                                },
                                span: span.clone(),
                            }
                            .into());
                        }
                        let mut unified_props = Vec::with_capacity(aprops.len());
                        for (prop_a, prop_b) in aprops.iter().zip(bprops.iter()) {
                            let unified_prop = self.unify(&prop_a.2, &prop_b.2, span)?;
                            unified_props.push((prop_a.0.clone(), prop_a.1.clone(), unified_prop));
                        }
                        let HirType::Component { props } = self.types_module.get_type_mut(a) else {
                            unreachable!()
                        };
                        props.clear();
                        props.extend_from_slice(&unified_props);
                        let HirType::Component { props } = self.types_module.get_type_mut(b) else {
                            unreachable!()
                        };
                        props.clear();
                        props.extend_from_slice(&unified_props);
                        Ok(*a)
                    }
                    (HirType::Component { .. }, HirType::GenericComponent) => Ok(*a),
                    (HirType::GenericComponent, HirType::Component { .. }) => Ok(*b),

                    (HirType::Struct { fields: f1 }, HirType::Struct { fields: f2 }) => {
                        if f1.len() != f2.len() {
                            Err(TypeError {
                                kind: TypeErrorKind::IncompatibleTypes {
                                    expected: concrete_a,
                                    received: concrete_b,
                                },
                                span: span.clone(),
                            }
                            .into())
                        } else {
                            for idx in 0..f1.len() {
                                self.unify(&f1[idx], &f2[idx], span)?;
                            }
                            Ok(*a)
                        }
                    }
                    (HirType::VarReference(rf1), HirType::VarReference(rf2)) => {
                        let Some(rf1) = self.types_module.get_variable(rf1).cloned() else {
                            unreachable!("Variable should have already been declared")
                        };
                        let Some(rf2) = self.types_module.get_variable(rf2).cloned() else {
                            unreachable!("Variable2 should have already been declared")
                        };
                        self.unify(&rf1, &rf2, span)
                    }

                    (_, _) => Err(TypeError {
                        kind: TypeErrorKind::IncompatibleTypes {
                            expected: concrete_b,
                            received: concrete_a,
                        },
                        span: span.clone(),
                    }
                    .into()),
                }
            }
        }
    }

    fn unify_with_ref(&mut self, rf: TypeId, ty: TypeId, span: &Span) -> Result<TypeId> {
        let resolved_ref = self.resolve(&rf, span)?;
        if !matches!(
            self.types_module.get_type(&resolved_ref),
            HirType::Reference { .. }
        ) {
            return self.unify(&resolved_ref, &ty, span);
        }
        if let HirType::Reference { rf: refe, .. } = self.types_module.get_type(&ty)
            && rf == *refe
        {
            let ty = self.types_module.insert_unnamed_type(HirType::Reference {
                rf: *refe,
                generics: Vec::new(),
            });
            return Ok(ty);
        }
        let ty = self.types_module.get_type(&ty);
        if self.reccursive_ty(rf, ty) {
            return Err(TypeError {
                kind: TypeErrorKind::CiclicType { ty: ty.clone() },
                span: span.clone(),
            }
            .into());
        }
        self.substitute(rf, ty.clone());
        let ty = self.types_module.insert_unnamed_type(HirType::Reference {
            rf,
            generics: Vec::new(),
        });
        Ok(ty)
    }

    /// Checks if the provided `ty` is recursive
    fn reccursive_ty(&self, ty_ref: TypeId, ty: &HirType) -> bool {
        match ty {
            HirType::Reference { rf, .. } => {
                if ty_ref == *rf {
                    true
                } else if let Some(resolved) = self.types.get(rf) {
                    self.reccursive_ty(ty_ref, resolved)
                } else {
                    false
                }
            }
            HirType::Component { props } => props
                .iter()
                .any(|prop| self.reccursive_ty(ty_ref, self.types_module.get_type(&prop.2))),
            _ => false,
        }
    }

    fn set_default(&mut self, decl: &mut HirDeclaration) -> Result<()> {
        match decl.kind {
            HirDeclarationKind::Object => {}
            HirDeclarationKind::Function {
                ref mut statements, ..
            } => {
                let HirType::Function { return_type, .. } =
                    self.types_module.get_type(&decl.ty).clone()
                else {
                    unreachable!("A function should have function type");
                };
                for statement in statements {
                    self.default_statement(statement, &return_type)?;
                }
            }
            HirDeclarationKind::ComponentDeclaration { ref mut props } => {
                self.resolve_component_members(props, decl.ty)?;
            }
        }
        Ok(())
    }
}
