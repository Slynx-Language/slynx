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

mod decl;
pub mod error;
mod expr;
mod statement;
use std::collections::HashMap;

use color_eyre::eyre::Result;
use common::SymbolPointer;

use crate::error::{IncompatibleComponentReason, TypeError, TypeErrorKind};

use common::ast::Span;
use slynx_hir::model::{ComponentProperty, HirStatement, HirStatementKind};
use slynx_hir::modules::TypesModule;
use slynx_hir::{
    SlynxHir, TypeId,
    model::{FieldMethod, HirDeclaration, HirDeclarationKind, HirType},
};
#[derive(Debug)]
///The type checker of slynx. Slynx has 2 phases of type checking by now. The first phase is to get information about the types on all the HIR
///and try to type them properly. Thus, the HIR might come with infers, and the type checker tries to get rid of them with their actual type. This is made with all function
///whose got `resolve` on their names. Since the idea is for them to try to resolve the types. The second part is that, after checking all the IR, if some code could not be infered,
///then it starts setting the default values for every of them. This is made with functions named as `default`. So every infers are get rid because the default type is provided.
pub struct TypeChecker {
    ///A an array of declaration types
    declarations: Vec<TypeId>,
    ///The types module from the HIR to be mutated
    types_module: TypesModule,
    /// The type of everything that is expected to have some
    types: HashMap<TypeId, HirType>,
    structs: HashMap<TypeId, Vec<SymbolPointer>>,
}

impl TypeChecker {
    fn get_object_layout_type(&self, ty: &TypeId, span: &Span) -> Result<TypeId> {
        let mut current = *ty;

        loop {
            match self.types_module.get_type(&current) {
                HirType::Reference { rf, .. } => match self.types_module.get_type(rf) {
                    HirType::Struct { .. } => return Ok(current),
                    HirType::Reference { .. } => current = *rf,
                    other => {
                        return Err(TypeError {
                            kind: TypeErrorKind::NotAStruct(other.clone()),
                            span: *span,
                        }
                        .into());
                    }
                },
                HirType::VarReference(variable_id) => {
                    current =
                        self.types_module
                            .get_variable(variable_id)
                            .copied()
                            .ok_or(TypeError {
                                kind: TypeErrorKind::Unrecognized,
                                span: *span,
                            })?;
                }
                other => {
                    return Err(TypeError {
                        kind: TypeErrorKind::NotAStruct(other.clone()),
                        span: *span,
                    }
                    .into());
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
            }
            .into());
        };

        fields.get(index).copied().ok_or(
            TypeError {
                kind: TypeErrorKind::InvalidTupleIndex {
                    index,
                    length: fields.len(),
                },
                span: *span,
            }
            .into(),
        )
    }

    /// Checks the types of the provided `hir` and mutates them if needed. Any that could not be inferred but, yet is valid, is
    /// at the end, returned as it's default type. Returns the type module to be used on the next steps
    pub fn check(hir: &mut SlynxHir) -> Result<TypesModule> {
        let mut inner = Self {
            types: HashMap::new(),
            structs: std::mem::take(&mut hir.modules.declarations_module.objects),
            types_module: std::mem::take(&mut hir.modules.types_module),
            declarations: Vec::new(),
        };
        // DeclarationId is currently assigned linearly in hoisting order,
        // so declaration type lookup can stay append-only here.
        for decl in &hir.declarations {
            debug_assert_eq!(
                decl.id.as_raw() as usize,
                inner.declarations.len(),
                "declaration id must stay linear with declarations table",
            );
            inner.declarations.push(decl.ty);
        }
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

    /// Resolves recursively the names of the types. If A -> B, B -> int; then we assume that A -> int
    fn resolve(&mut self, ty: &TypeId, span: &Span) -> Result<TypeId> {
        let referedty = self.types_module.get_type(ty).clone();
        match referedty {
            HirType::Field(FieldMethod::Type(rf, index)) => {
                let ty = self.resolve(&rf, span)?;
                let struct_id = self.get_struct_from_ref(&ty, span)?;
                if let HirType::Struct { fields } = self.types_module.get_type(&struct_id) {
                    Ok(fields[index])
                } else {
                    Err(TypeError {
                        kind: TypeErrorKind::IncompatibleTypes {
                            expected: self.types_module.get_type(&ty).clone(),
                            received: HirType::Struct { fields: Vec::new() },
                        },
                        span: *span,
                    }
                    .into())
                }
            }
            HirType::Field(FieldMethod::Tuple(rf, index)) => {
                self.resolve_tuple_index_type(&rf, index, span)
            }
            HirType::Field(FieldMethod::Variable(var_id, n)) => {
                let object_ty = *self.types_module.get_variable(&var_id).ok_or(TypeError {
                    kind: TypeErrorKind::Unrecognized,
                    span: *span,
                })?;
                let layout_ty = self.get_object_layout_type(&object_ty, span)?;
                let concrete_type = self
                    .types_module
                    .get_type(&self.get_struct_from_ref(&object_ty, span)?);
                let s_fields = self.structs.get(&layout_ty).ok_or(TypeError {
                    kind: TypeErrorKind::Unrecognized,
                    span: *span,
                })?;
                let HirType::Struct { fields } = concrete_type else {
                    unreachable!("Type should be a struct. Fields only happen to structs");
                };
                if let Some(index) = s_fields.iter().position(|name| *name == n) {
                    Ok(fields[index])
                } else {
                    Err(TypeError {
                        kind: TypeErrorKind::Unrecognized,
                        span: *span,
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
                        let mut prop = prop.clone();
                        *prop.prop_type_mut() = self.resolve(prop.prop_type(), span)?;
                        tys.push(prop)
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
                                span: *span,
                            }
                            .into());
                        }
                        let mut unified_props = Vec::with_capacity(aprops.len());
                        for (prop_a, prop_b) in aprops.iter().zip(bprops.iter()) {
                            let unified_prop =
                                self.unify(prop_a.prop_type(), prop_b.prop_type(), span)?;
                            unified_props.push(ComponentProperty::new(
                                *prop_a.visibility(),
                                prop_a.name().to_string(),
                                unified_prop,
                            ));
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
                                span: *span,
                            }
                            .into())
                        } else {
                            for idx in 0..f1.len() {
                                self.unify(&f1[idx], &f2[idx], span)?;
                            }
                            Ok(*a)
                        }
                    }
                    (HirType::Tuple { fields: f1 }, HirType::Tuple { fields: f2 }) => {
                        if f1.len() != f2.len() {
                            Err(TypeError {
                                kind: TypeErrorKind::IncompatibleTypes {
                                    expected: concrete_a,
                                    received: concrete_b,
                                },
                                span: *span,
                            }
                            .into())
                        } else {
                            let mut new_fields = Vec::with_capacity(f1.len());

                            for (t1, t2) in f1.iter().zip(f2.iter()) {
                                let unified = self.unify(t1, t2, span)?;
                                new_fields.push(unified);
                            }

                            Ok(self.types_module.add_tuple_type(new_fields))
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
                        span: *span,
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
            return Ok(ty);
        }
        let ty = self.types_module.get_type(&ty);
        if self.recursive_ty(rf, ty) {
            return Err(TypeError {
                kind: TypeErrorKind::CiclicType { ty: ty.clone() },
                span: *span,
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
    fn recursive_ty(&self, ty_ref: TypeId, ty: &HirType) -> bool {
        match ty {
            HirType::Reference { rf, .. } => {
                if ty_ref == *rf {
                    true
                } else if let Some(resolved) = self.types.get(rf) {
                    self.recursive_ty(ty_ref, resolved)
                } else {
                    false
                }
            }
            HirType::Component { props } => props.iter().any(|prop| {
                self.recursive_ty(ty_ref, self.types_module.get_type(prop.prop_type()))
            }),
            _ => false,
        }
    }

    ///Sets the default types on the given `decl`. This replaces the infer types on everything on the given `decl` with the correct(or default) type
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
                for statement in &mut *statements {
                    self.default_statement(statement, &return_type)?;
                }
                self.ensure_function_returns(statements, return_type, &decl.span)?;
            }
            HirDeclarationKind::ComponentDeclaration { ref mut props, .. } => {
                self.resolve_component_members(props, decl.ty)?;
            }
            HirDeclarationKind::Alias => {}
        }
        Ok(())
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
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::TypeChecker;
    use crate::{
        checker::error::{TypeError, TypeErrorKind},
        hir::{
            ExpressionId, SlynxHir,
            model::{
                HirDeclarationKind, HirExpression, HirExpressionKind, HirStatementKind, HirType,
            },
        },
        lexer::Lexer,
        parser::Parser,
    };

    fn load_hir_from_source(source: &str) -> SlynxHir {
        let tokens = Lexer::tokenize(source).expect("source should tokenize");
        let declarations = Parser::new(tokens)
            .parse_declarations()
            .expect("source should parse");
        let mut hir = SlynxHir::new();
        hir.generate(declarations).expect("HIR should generate");
        hir
    }

    fn load_hir(path: &str) -> SlynxHir {
        let source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
        let source = std::fs::read_to_string(&source_path).expect("source file should exist");
        load_hir_from_source(&source)
    }

    fn main_call_args(hir: &mut SlynxHir) -> Option<&mut Vec<HirExpression>> {
        for declaration in &mut hir.declarations {
            let HirDeclarationKind::Function {
                name, statements, ..
            } = &mut declaration.kind
            else {
                continue;
            };

            if let Some(main_symbol) = hir.modules.retrieve_symbol("main")
                && main_symbol == *name
            {
                continue;
            }

            for statement in statements {
                let expr = match &mut statement.kind {
                    HirStatementKind::Variable { value, .. } => value,
                    HirStatementKind::Expression { expr } => expr,
                    HirStatementKind::Return { expr } => expr,
                    HirStatementKind::Assign { value, .. } => value,
                    HirStatementKind::While { .. } => continue,
                };
                let HirExpressionKind::FunctionCall { args, .. } = &mut expr.kind else {
                    continue;
                };
                return Some(args);
            }
        }
        None
    }

    #[test]
    fn function_calls_work_with_mixed_declaration_order() {
        let mut hir = load_hir("slynx/functioncall.slynx");
        TypeChecker::check(&mut hir).expect("function call should resolve with declaration ids");
    }

    #[test]
    fn rejects_function_call_with_missing_arg() {
        let mut hir = load_hir("slynx/functioncall.slynx");
        let args = main_call_args(&mut hir).expect("expected to find a function call in main");
        args.pop();

        let err = TypeChecker::check(&mut hir)
            .expect_err("type checker should reject function calls with missing args");
        let type_error = err
            .downcast_ref::<TypeError>()
            .expect("error should come from type checker");

        match &type_error.kind {
            TypeErrorKind::InvalidFuncallArgLength {
                expected_length,
                received_length,
            } => {
                assert_eq!(*expected_length, 2);
                assert_eq!(*received_length, 1);
            }
            other => panic!("expected InvalidFuncallArgLength, got {other:?}"),
        }
    }

    #[test]
    fn rejects_function_call_with_extra_arg() {
        let mut hir = load_hir("slynx/functioncall.slynx");
        let args = main_call_args(&mut hir).expect("expected to find a function call in main");
        let template = args.first().expect("call should have at least one arg");
        args.push(HirExpression {
            id: ExpressionId::new(),
            ty: template.ty,
            kind: HirExpressionKind::Int(0),
            span: template.span.clone(),
        });

        let err = TypeChecker::check(&mut hir)
            .expect_err("type checker should reject function calls with extra args");
        let type_error = err
            .downcast_ref::<TypeError>()
            .expect("error should come from type checker");

        match &type_error.kind {
            TypeErrorKind::InvalidFuncallArgLength {
                expected_length,
                received_length,
            } => {
                assert_eq!(*expected_length, 2);
                assert_eq!(*received_length, 3);
            }
            other => panic!("expected InvalidFuncallArgLength, got {other:?}"),
        }
    }

    #[test]
    fn rejects_function_call_with_wrong_argument_type() {
        let mut hir = load_hir("slynx/functioncall.slynx");
        let bool_ty = hir.bool_type();
        let args = main_call_args(&mut hir).expect("expected to find a function call in main");
        let first_arg = args.first_mut().expect("call should have at least one arg");
        first_arg.kind = HirExpressionKind::Bool(true);
        first_arg.ty = bool_ty;

        let err = TypeChecker::check(&mut hir)
            .expect_err("type checker should reject function calls with wrong arg type");
        let type_error = err
            .downcast_ref::<TypeError>()
            .expect("error should come from type checker");

        assert!(
            matches!(type_error.kind, TypeErrorKind::IncompatibleTypes { .. }),
            "expected IncompatibleTypes, got {:?}",
            type_error.kind
        );
    }

    #[test]

    fn rejects_function_without_return_value_for_non_void_return_type() {
        let mut hir = load_hir_from_source(
            r#"
            func main(): int {
                let x = 12;
            }
            "#,
        );

        let err = TypeChecker::check(&mut hir).expect_err("non-void functions must return a value");
        let type_error = err
            .downcast_ref::<TypeError>()
            .expect("error should come from type checker");

        match &type_error.kind {
            TypeErrorKind::MissingReturnValue { expected } => {
                assert!(
                    matches!(expected, HirType::Int),
                    "expected missing int return, got {expected:?}"
                );
            }
            other => panic!("expected MissingReturnValue, got {other:?}"),
        }
    }

    #[test]
    fn preserves_non_expression_tail_statement_in_function_body() {
        let hir = load_hir_from_source(
            r#"
            func main(): void {
                let x = 12;
            }
            "#,
        );
        let main_symbol = hir
            .modules
            .symbols_resolver
            .retrieve("main")
            .copied()
            .expect("main symbol should exist");

        let main_fn = hir
            .declarations
            .iter()
            .find(|declaration| {
                matches!(
                    declaration.kind,
                    HirDeclarationKind::Function { ref name, .. } if *name == main_symbol
                )
            })
            .expect("main function should exist");

        let HirDeclarationKind::Function { statements, .. } = &main_fn.kind else {
            unreachable!();
        };

        assert_eq!(
            statements.len(),
            1,
            "last non-expression statement should be preserved"
        );
        assert!(
            matches!(statements[0].kind, HirStatementKind::Variable { .. }),
            "expected trailing let statement to stay in the body"
        );
    }

    #[test]
    fn rejects_while_with_non_boolean_condition() {
        let mut hir = load_hir_from_source(
            r#"
            func main(): void {
                while 10 {
                    0;
                }
            }
            "#,
        );

        let err = TypeChecker::check(&mut hir).expect_err("while conditions should require bool");
        let type_error = err
            .downcast_ref::<TypeError>()
            .expect("error should come from type checker");

        assert!(
            matches!(type_error.kind, TypeErrorKind::IncompatibleTypes { .. }),
            "expected IncompatibleTypes, got {:?}",
            type_error.kind
        );
    }

    #[test]
    fn rejects_invalid_statement_inside_while_body() {
        let mut hir = load_hir_from_source(
            r#"
            func takes_int(value: int): void {}

            func main(): void {
                while true {
                    takes_int(false);
                }
            }
            "#,
        );

        let err =
            TypeChecker::check(&mut hir).expect_err("while body statements should be type-checked");
        let type_error = err
            .downcast_ref::<TypeError>()
            .expect("error should come from type checker");

        assert!(
            matches!(type_error.kind, TypeErrorKind::IncompatibleTypes { .. }),
            "expected IncompatibleTypes, got {:?}",
            type_error.kind
        );
    }

    #[test]
    fn resolves_field_access_for_variables_typed_via_alias() {
        let mut hir = load_hir_from_source(
            r#"
            object Person {
                age: int,
            }

            alias PersonAlias = Person;

            func make_person(): PersonAlias {
                Person(age: 22)
            }

            func main(): int {
                let person = make_person();
                person.age
            }
            "#,
        );

        TypeChecker::check(&mut hir).expect("field access should resolve through aliases");
    }

    #[test]
    fn resolves_tuple_access_for_tuple_variables() {
        let mut hir = load_hir_from_source(
            r#"
            func main(): int {
                let pair = (10, 20);
                pair.0
            }
            "#,
        );

        TypeChecker::check(&mut hir).expect("tuple access should resolve through the checker");
    }

    #[test]
    fn resolves_named_field_access_after_tuple_access() {
        let mut hir = load_hir_from_source(
            r#"
            object Person {
                age: int,
            }

            func main(): int {
                let pair = (Person(age: 22), "ok");
                pair.0.age
            }
            "#,
        );

        TypeChecker::check(&mut hir)
            .expect("named field access after tuple access should resolve cleanly");
    }

    #[test]
    fn rejects_tuple_access_with_invalid_index() {
        let mut hir = load_hir_from_source(
            r#"
            func main(): int {
                let pair = (10, 20);
                pair.2
            }
            "#,
        );

        let err =
            TypeChecker::check(&mut hir).expect_err("tuple accesses should reject invalid indexes");
        let type_error = err
            .downcast_ref::<TypeError>()
            .expect("error should come from type checker");

        match &type_error.kind {
            TypeErrorKind::InvalidTupleIndex { index, length } => {
                assert_eq!(*index, 2);
                assert_eq!(*length, 2);
            }
            other => panic!("expected InvalidTupleIndex, got {other:?}"),
        }
    }

    #[test]
    fn rejects_tuple_access_on_non_tuple_values() {
        let mut hir = load_hir_from_source(
            r#"
            func main(): int {
                let value = 10;
                value.0
            }
            "#,
        );

        let err =
            TypeChecker::check(&mut hir).expect_err("non-tuples should reject tuple-style access");
        let type_error = err
            .downcast_ref::<TypeError>()
            .expect("error should come from type checker");

        assert!(
            matches!(
                type_error.kind,
                TypeErrorKind::InvalidTupleAccessTarget { .. }
            ),
            "expected InvalidTupleAccessTarget, got {:?}",
            type_error.kind
        );
    }
}
