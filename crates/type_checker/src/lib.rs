//! The core type-checking engine for the Slynx compiler.
//!
//! This module implements a two-phase type inference and validation system:
//!
//! 1. **Resolution Phase**: Iterates through the HIR to resolve `Infer` types
//!    into concrete types by analyzing expressions, function calls, and
//!    component properties. Functions involved in this phase are typically
//!    named `resolve_*`.
//!
//! 2. **Default/Fallback Phase**: After the initial pass, any remaining
//!    ambiguous types that couldn't be strictly inferred are assigned their
//!    language-default types. Functions for this phase are named `set_default`
//!    or `default_*`.
//!
//! The `TypeChecker` mutates the `TypesModule` and the `SlynxHir` in-place,
//! ensuring that subsequent compilation stages (like Codegen) have access to
//! fully concrete type information.

mod declaration;
mod defaults;
pub mod error;
pub use error::{IncompatibleComponentReason, TypeError, TypeErrorKind};
use slynx_hir::context::TypesContext;
mod expression;
mod statement;
mod styles;
use std::collections::HashMap;

use common::Span;
use slynx_hir::{ComponentProperty, DeclarationId, HirStatement, HirStatementKind};
use slynx_hir::{FieldMethod, HirType, SlynxHir, TypeId};

pub type Result<T> = std::result::Result<T, TypeError>;

#[derive(Debug)]
///The type checker of slynx. Slynx has 2 phases of type checking by now. The first phase is to get information about the types on all the HIR
///and try to type them properly. Thus, the HIR might come with infers, and the type checker tries to get rid of them with their actual type. This is made with all function
///whose got `resolve` on their names. Since the idea is for them to try to resolve the types. The second part is that, after checking all the IR, if some code could not be infered,
///then it starts setting the default values for every of them. This is made with functions named as `default`. So every infers are get rid because the default type is provided.
pub struct TypeChecker {
    ///A map of declaration types keyed by declaration ID
    decl_types: HashMap<DeclarationId, TypeId>,
    ///The types module from the HIR to be mutated
    types_module: TypesContext,
    /// The type of everything that is expected to have some
    types: HashMap<TypeId, HirType>,
}

impl TypeChecker {
    fn get_object_layout_type(&self, ty: &TypeId, span: &Span) -> Result<TypeId> {
        let mut current = *ty;

        loop {
            let ty = self.types_module.get_type(&current).clone();
            match ty {
                HirType::Reference { rf, .. } => {
                    let inner = self.types_module.get_type(&rf).clone();
                    match inner {
                        HirType::Struct { .. } => return Ok(current),
                        HirType::Reference { .. } => current = rf,
                        other => {
                            return Err(TypeError {
                                kind: TypeErrorKind::NotAStruct(other),
                                span: *span,
                            });
                        }
                    }
                }
                HirType::VarReference(variable_id) => {
                    current = self
                        .types_module
                        .get_variable(&variable_id)
                        .ok_or(TypeError {
                            kind: TypeErrorKind::Unrecognized,
                            span: *span,
                        })?;
                }
                other => {
                    return Err(TypeError {
                        kind: TypeErrorKind::NotAStruct(other),
                        span: *span,
                    });
                }
            }
        }
    }

    fn resolve_tuple_index_type(
        &mut self,
        ty: &TypeId,
        index: usize,
        span: &Span,
    ) -> Result<TypeId> {
        // Tuple access stays explicit all the way to the checker so numeric
        // postfixes never get confused with object field lookups.
        let resolved_ty = self.resolve(ty, span)?;
        let HirType::Tuple { fields } = self.types_module.get_type(&resolved_ty).clone() else {
            return Err(TypeError {
                kind: TypeErrorKind::InvalidTupleAccessTarget {
                    received: self.types_module.get_type(&resolved_ty).clone(),
                },
                span: *span,
            });
        };

        fields.get(index).copied().ok_or(TypeError {
            kind: TypeErrorKind::InvalidTupleIndex {
                index,
                length: fields.len(),
            },
            span: *span,
        })
    }

    /// Checks the types of the provided `hir` and mutates them in place. Any that could not be inferred but, yet is valid, is
    /// at the end, returned as it's default type. Returns the same `hir` with fully-resolved types.
    pub fn check(mut hir: SlynxHir) -> Result<SlynxHir> {
        let mut decl_types = HashMap::new();
        for file in &hir.files {
            let file = file.read();
            for decl in file.declarations() {
                let decl = decl.1;
                decl_types.insert(decl.id, decl.ty);
            }
        }

        let mut inner = Self {
            types: HashMap::new(),
            types_module: std::mem::take(&mut hir.types_module),
            decl_types,
        };

        // Phase 1: resolve infer types
        for file in &mut hir.files {
            let mut file = file.write();
            for idx in 0..file.declarations().count() {
                let decl = file
                    .declarations
                    .declarations
                    .get_mut(idx)
                    .expect("Declaration should've been initialized");
                inner.check_decl(decl)?;
            }
        }
        // Phase 2: set defaults for remaining infer types
        for file in &mut hir.files {
            let mut file = file.write();
            for idx in 0..file.declarations().count() {
                let decl = file
                    .declarations
                    .declarations
                    .get_mut(idx)
                    .expect("Declaration should've been initialized");
                inner.set_default(decl)?;
            }
        }

        hir.types_module = inner.types_module;
        Ok(hir)
    }

    fn substitute(&mut self, id: TypeId, ty: HirType) {
        self.types.insert(id, ty);
    }

    /// Resolves recursively the names of the types. If A -> B, B -> int; then we assume that A -> int
    fn resolve(&mut self, ty: &TypeId, span: &Span) -> Result<TypeId> {
        let referedty = self.types_module.get_type(ty).clone();
        match referedty {
            HirType::Field(FieldMethod::Type(rf, index)) => {
                let ty = self.resolve(&rf, span)?;
                let struct_id = self.get_struct_from_ref(&ty, span)?;
                if let HirType::Struct { fields } = &*self.types_module.get_type(&struct_id) {
                    Ok(fields[index])
                } else {
                    Err(TypeError {
                        kind: TypeErrorKind::IncompatibleTypes {
                            expected: self.types_module.get_type(&ty).clone(),
                            received: HirType::Struct { fields: Vec::new() },
                        },
                        span: *span,
                    })
                }
            }
            HirType::Field(FieldMethod::Tuple(rf, index)) => {
                self.resolve_tuple_index_type(&rf, index, span)
            }
            HirType::Field(FieldMethod::Variable(var_id, n)) => {
                let object_ty = self.types_module.get_variable(&var_id).ok_or(TypeError {
                    kind: TypeErrorKind::Unrecognized,
                    span: *span,
                })?;
                let layout_ty = self.get_object_layout_type(&object_ty, span)?;
                let concrete_type = self
                    .types_module
                    .get_type(&self.get_struct_from_ref(&object_ty, span)?);
                let s_fields = self.types_module.objects.get(&layout_ty).ok_or(TypeError {
                    kind: TypeErrorKind::Unrecognized,
                    span: *span,
                })?;
                let HirType::Struct { fields } = &*concrete_type else {
                    unreachable!("Type should be a struct. Fields only happen to structs");
                };
                if let Some(index) = s_fields.iter().position(|name| *name == n) {
                    Ok(fields[index])
                } else {
                    Err(TypeError {
                        kind: TypeErrorKind::Unrecognized,
                        span: *span,
                    })
                }
            }
            HirType::Reference { rf, .. } => Ok(rf),
            HirType::VarReference(rf) => {
                if let Some(ty) = self.types_module.get_variable(&rf) {
                    Ok(self.resolve(&ty, span)?)
                } else {
                    unreachable!("Variable type should be defined here");
                }
            }
            HirType::Component { props } => {
                let mut resolved_props = {
                    let mut tys = Vec::with_capacity(props.len());
                    for prop in props {
                        let mut prop = prop.clone();
                        *prop.prop_type_mut() = self.resolve(prop.prop_type(), span)?;
                        tys.push(prop)
                    }
                    tys
                };
                let HirType::Component { props } = &mut *self.types_module.get_type_mut(*ty) else {
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
                    (HirType::Component { .. }, HirType::Component { .. }) => {
                        self.unify_components(a, b, &concrete_a, &concrete_b, span)
                    }
                    (HirType::Component { .. }, HirType::GenericComponent) => Ok(*a),
                    (HirType::GenericComponent, HirType::Component { .. }) => Ok(*b),
                    (HirType::Struct { .. }, HirType::Struct { .. }) => {
                        self.unify_structs(a, &concrete_a, &concrete_b, span)
                    }
                    (HirType::Tuple { .. }, HirType::Tuple { .. }) => {
                        self.unify_tuples(&concrete_a, &concrete_b, span)
                    }
                    (HirType::VarReference(..), HirType::VarReference(..)) => {
                        self.unify_var_references(&concrete_a, &concrete_b, span)
                    }
                    (_, _) => Err(TypeError {
                        kind: TypeErrorKind::IncompatibleTypes {
                            expected: concrete_b,
                            received: concrete_a,
                        },
                        span: *span,
                    }),
                }
            }
        }
    }

    fn unify_components(
        &mut self,
        a: &TypeId,
        b: &TypeId,
        concrete_a: &HirType,
        concrete_b: &HirType,
        span: &Span,
    ) -> Result<TypeId> {
        let HirType::Component { props: aprops } = concrete_a else {
            unreachable!()
        };
        let HirType::Component { props: bprops } = concrete_b else {
            unreachable!()
        };

        if aprops.len() != bprops.len() {
            return Err(TypeError {
                kind: TypeErrorKind::IncompatibleComponent {
                    reason: IncompatibleComponentReason::DifferentPropAmount {
                        rhs: aprops.len(),
                        lhs: bprops.len(),
                    },
                },
                span: *span,
            });
        }
        let mut unified_props = Vec::with_capacity(aprops.len());
        for (prop_a, prop_b) in aprops.iter().zip(bprops.iter()) {
            let unified_prop = self.unify(prop_a.prop_type(), prop_b.prop_type(), span)?;
            unified_props.push(ComponentProperty::new(
                *prop_a.visibility(),
                prop_a.name(),
                unified_prop,
            ));
        }
        if let HirType::Component { props } = &mut *self.types_module.get_type_mut(*a) {
            props.clear();
            props.extend_from_slice(&unified_props);
        }
        if let HirType::Component { props } = &mut *self.types_module.get_type_mut(*b) {
            props.clear();
            props.extend_from_slice(&unified_props);
        }
        Ok(*a)
    }

    fn unify_structs(
        &mut self,
        a: &TypeId,
        concrete_a: &HirType,
        concrete_b: &HirType,
        span: &Span,
    ) -> Result<TypeId> {
        let HirType::Struct { fields: f1 } = concrete_a else {
            unreachable!()
        };
        let HirType::Struct { fields: f2 } = concrete_b else {
            unreachable!()
        };

        if f1.len() != f2.len() {
            return Err(TypeError {
                kind: TypeErrorKind::IncompatibleTypes {
                    expected: concrete_a.clone(),
                    received: concrete_b.clone(),
                },
                span: *span,
            });
        }
        for idx in 0..f1.len() {
            self.unify(&f1[idx], &f2[idx], span)?;
        }
        Ok(*a)
    }

    fn unify_tuples(
        &mut self,
        concrete_a: &HirType,
        concrete_b: &HirType,
        span: &Span,
    ) -> Result<TypeId> {
        let HirType::Tuple { fields: f1 } = concrete_a else {
            unreachable!()
        };
        let HirType::Tuple { fields: f2 } = concrete_b else {
            unreachable!()
        };

        if f1.len() != f2.len() {
            return Err(TypeError {
                kind: TypeErrorKind::IncompatibleTypes {
                    expected: concrete_a.clone(),
                    received: concrete_b.clone(),
                },
                span: *span,
            });
        }
        let mut new_fields = Vec::with_capacity(f1.len());
        for (t1, t2) in f1.iter().zip(f2.iter()) {
            let unified = self.unify(t1, t2, span)?;
            new_fields.push(unified);
        }
        Ok(self.types_module.create_tuple_type(new_fields))
    }

    fn unify_var_references(
        &mut self,
        concrete_a: &HirType,
        concrete_b: &HirType,
        span: &Span,
    ) -> Result<TypeId> {
        let HirType::VarReference(var_a) = concrete_a else {
            unreachable!()
        };
        let HirType::VarReference(var_b) = concrete_b else {
            unreachable!()
        };

        let Some(resolved_a) = self.types_module.get_variable(var_a) else {
            unreachable!("Variable should have already been declared")
        };
        let Some(resolved_b) = self.types_module.get_variable(var_b) else {
            unreachable!("Variable2 should have already been declared")
        };
        self.unify(&resolved_a, &resolved_b, span)
    }

    fn unify_with_ref(&mut self, rf: TypeId, ty: TypeId, span: &Span) -> Result<TypeId> {
        let resolved_ref = self.resolve(&rf, span)?;
        if !matches!(
            &*self.types_module.get_type(&resolved_ref),
            HirType::Reference { .. }
        ) {
            return self.unify(&resolved_ref, &ty, span);
        }
        if let HirType::Reference { rf: refe, .. } = &*self.types_module.get_type(&ty)
            && rf == *refe
        {
            return Ok(ty);
        }
        let ty_hir = self.types_module.get_type(&ty).clone();
        if self.is_recursive_type(rf, &ty_hir) {
            return Err(TypeError {
                kind: TypeErrorKind::CyclicType { ty: ty_hir },
                span: *span,
            });
        }
        self.substitute(rf, ty_hir);
        let ty = self.types_module.create_unnamed_type(HirType::Reference {
            rf,
            generics: Vec::new(),
        });
        Ok(ty)
    }

    /// Checks if the provided `ty` is recursive
    fn is_recursive_type(&self, ty_ref: TypeId, ty: &HirType) -> bool {
        match ty {
            HirType::Reference { rf, .. } => {
                if ty_ref == *rf {
                    true
                } else if let Some(resolved) = self.types.get(rf) {
                    self.is_recursive_type(ty_ref, resolved)
                } else {
                    false
                }
            }
            HirType::Component { props } => props.iter().any(|prop| {
                self.is_recursive_type(ty_ref, &*self.types_module.get_type(prop.prop_type()))
            }),
            _ => false,
        }
    }

    fn ensure_function_returns(
        &mut self,
        statements: &[HirStatement],
        return_type: TypeId,
        span: &Span,
    ) -> Result<()> {
        let return_type = self.resolve(&return_type, span)?;
        if return_type == self.types_module.void_id() {
            return Ok(());
        }

        // Functions return implicitly through their tail expression today,
        // so a non-void function must end with a lowered `Return`.
        if matches!(
            statements.last().map(|statement| &statement.kind),
            Some(HirStatementKind::Return { .. })
        ) {
            return Ok(());
        }

        Err(TypeError {
            kind: TypeErrorKind::MissingReturnValue {
                expected: self.types_module.get_type(&return_type).clone(),
            },
            span: *span,
        })
    }
}
