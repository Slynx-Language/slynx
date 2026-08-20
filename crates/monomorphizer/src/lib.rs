//! Monomorphization.
//!
//! The pass turns generic declarations into concrete ones. Generic *functions*
//! are specialized at explicit call sites (`identity<int>(x)`); generic
//! *structs* (objects) and *components* are specialized wherever a concrete
//! type application appears (`Option<int>`, `List<int>`). Every generic
//! template that is not itself concrete is neutralized and reported as dead
//! code, so codegen never sees a [`HirType::GenericParam`].
//!
//! # File organization
//!
//! The pass is split per declaration kind:
//!
//! - [`functions`](crate)::[`functions`](self::functions) — function specialization.
//! - [`structs`](self::structs) — struct (object) specialization.
//! - [`components`](self::components) — component specialization.
//! - [`types`](self::types) — shared type-substitution machinery.
//!
//! See `docs/architecture.md` in this crate for the full picture, and
//! `docs/extension-guide.md` for how to add another declaration kind.
//!
//! The [`Monomorphizer`] struct owns the shared state (memoization cache,
//! in-progress set for cycle detection, and the dead-code set) and the
//! declaration-agnostic tree builders that every kind reuses.

mod components;
mod functions;
mod structs;
mod types;

use std::collections::{HashMap, HashSet};

use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use dashmap::DashMap;
use module_loader::FileId;
use slynx_hir::{
    DeclarationId, HirComponentExpression, HirExpression, HirExpressionKind,
    HirFunctionDeclaration, HirStatement, HirType, PropertyExpression, Result, SlynxHir,
    VariableId,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};

use types::{
    MonomorphizationKey, Substitution, contains_resolvable_reference, is_resolvable_reference,
    substitute_type,
};

/// A snapshot of a function to be rewritten: its local declaration id and the
/// ids of its statements.
type FunctionSnapshot = (
    PoolId<HirFunctionDeclaration>,
    Vec<Spanned<PoolId<HirStatement>>>,
);

/// The type information tracked for a `let`-bound variable while its scope is
/// being rewritten.
///
/// - `original` is the type the initializer expression had *before*
///   monomorphization. When the variable has no type annotation this is also
///   the type every identifier referencing it carries (a `GenericParam` that
///   the empty substitution cannot replace).
/// - `rebuilt` is the concrete type of the rebuilt initializer, which is the
///   variable's real type once the call/object it came from was specialized.
#[derive(Clone, Copy)]
struct TrackedVariable {
    original: DedupPoolId<HirType>,
    rebuilt: DedupPoolId<HirType>,
}

/// A struct that handles all the monomorphization on the code.
///
/// Monomorphization specializes generic declarations by instantiating them with
/// the concrete type arguments requested at use sites. Every use of a generic
/// declaration with explicit type arguments is rewritten to point at a freshly
/// generated, concrete (non-generic) copy.
///
/// The original generic templates are kept in the HIR but are *neutralized*
/// (empty body, `()->void` signature) so downstream passes that cannot handle
/// `HirType::GenericParam` keep working. The set of neutralized templates is
/// returned by [`resolve`](Monomorphizer::resolve) as dead code for later
/// consumption by the code generation pipeline.
pub struct Monomorphizer {
    /// Already generated specializations. Mapping from `(template, type_args)`
    /// to the id of the generated concrete declaration.
    cache: DashMap<MonomorphizationKey, AnyDeclarationId>,
    /// Instantiations currently being generated. Used to detect
    /// non-terminating instantiations (a key that re-enters itself while still
    /// in progress).
    in_progress: HashSet<MonomorphizationKey>,
    /// The generic templates that were neutralized and are now dead code.
    dead_code: HashSet<AnyDeclarationId>,
    /// Stack of lexical scopes currently being rewritten. Each scope maps a
    /// `let`-bound variable to the type its (rebuilt) initializer produced, so
    /// later identifiers resolve to the concrete type instead of a leftover
    /// `GenericParam`.
    variable_types: Vec<HashMap<VariableId, TrackedVariable>>,
}

impl Monomorphizer {
    /// Monomorphizes every generic function, struct (object), and component of
    /// the given `hir`.
    ///
    /// Generic type aliases and stylesheets are currently unsupported and will
    /// `unimplemented!()`.
    ///
    /// Returns the set of generic templates that were neutralized and should
    /// be treated as dead code.
    pub fn resolve(hir: &mut SlynxHir) -> Result<HashSet<AnyDeclarationId>> {
        let mut monomorphizer = Self {
            cache: DashMap::new(),
            in_progress: HashSet::new(),
            dead_code: HashSet::new(),
            variable_types: Vec::new(),
        };
        monomorphizer.run(hir)?;
        Ok(monomorphizer.dead_code)
    }

    fn run(&mut self, hir: &SlynxHir) -> Result<()> {
        self.assert_no_generic_non_functions(hir);

        let files: Vec<FileId> = hir.files.iter().map(|file| *file.key()).collect();

        // Step 1: rewrite every non-generic function body, resolving generic
        // call sites and generic struct/component usage as they are found.
        // Specializations may discover further generic usage and instantiate it
        // recursively.
        for file_id in &files {
            let targets: Vec<FunctionSnapshot> = {
                let file = hir.get_file(*file_id);
                file.declarations
                    .declarations
                    .functions
                    .iter()
                    .with_ids()
                    .filter(|(_, declaration)| declaration.generics.is_empty())
                    .map(|(id, declaration)| (id, declaration.statements.clone()))
                    .collect()
            };

            for (local_id, statements) in targets {
                let new_statements =
                    self.build_statements(hir, &statements, &Substitution::empty())?;
                let mut file = hir.get_file_mut(*file_id);
                file.declarations
                    .declarations
                    .functions
                    .get_mut(local_id)
                    .statements = new_statements;
            }
        }

        // Step 2: rewrite the members (property defaults and child tree) of
        // every non-generic component.
        for file_id in &files {
            let ids: Vec<PoolId<slynx_hir::HirComponentDeclaration>> = {
                let file = hir.get_file(*file_id);
                file.declarations
                    .declarations
                    .components
                    .iter()
                    .with_ids()
                    .filter(|(_, declaration)| declaration.generics.is_empty())
                    .map(|(id, _)| id)
                    .collect()
            };

            for local_id in ids {
                self.rewrite_non_generic_component(hir, *file_id, local_id)?;
            }
        }

        // Step 3: resolve generic struct/component references in the signatures
        // of non-generic functions and components.
        for file_id in &files {
            let function_ids: Vec<PoolId<HirFunctionDeclaration>> = {
                let file = hir.get_file(*file_id);
                file.declarations
                    .declarations
                    .functions
                    .iter()
                    .with_ids()
                    .filter(|(_, declaration)| declaration.generics.is_empty())
                    .map(|(id, _)| id)
                    .collect()
            };
            for local_id in function_ids {
                let old_ty =
                    hir.get_file(*file_id).declarations.declarations.functions[local_id].ty;
                if contains_resolvable_reference(hir, old_ty) {
                    let new_ty = self.resolve_expression_type(hir, old_ty, Span::default())?;
                    hir.get_file_mut(*file_id)
                        .declarations
                        .declarations
                        .functions
                        .get_mut(local_id)
                        .ty = new_ty;
                }
            }

            let component_ids: Vec<PoolId<slynx_hir::HirComponentDeclaration>> = {
                let file = hir.get_file(*file_id);
                file.declarations
                    .declarations
                    .components
                    .iter()
                    .with_ids()
                    .filter(|(_, declaration)| declaration.generics.is_empty())
                    .map(|(id, _)| id)
                    .collect()
            };
            for local_id in component_ids {
                let old_ty =
                    hir.get_file(*file_id).declarations.declarations.components[local_id].ty;
                if contains_resolvable_reference(hir, old_ty) {
                    let new_ty = self.resolve_expression_type(hir, old_ty, Span::default())?;
                    hir.get_file_mut(*file_id)
                        .declarations
                        .declarations
                        .components
                        .get_mut(local_id)
                        .ty = new_ty;
                }
            }
        }

        // Step 4: neutralize every generic template so codegen never sees a
        // `GenericParam`-typed signature, and mark it as dead.
        let void_ty = hir.create_function_type(Vec::new(), hir.create_type(HirType::Void));
        self.neutralize_generic_functions(hir, &files, void_ty);
        self.neutralize_generic_objects(hir, &files, void_ty);
        self.neutralize_generic_components(hir, &files, void_ty);

        Ok(())
    }

    ///Monomorphization of generic type aliases and stylesheets is not supported
    ///yet, so encountering one is a hard error (`unimplemented!()`).
    fn assert_no_generic_non_functions(&self, hir: &SlynxHir) {
        for file in hir.files.iter() {
            for alias in file.declarations.declarations.alias.iter() {
                if !alias.generics.is_empty() {
                    unimplemented!("monomorphization of generic aliases is not implemented yet")
                }
            }
            for style in file.declarations.declarations.styles.iter() {
                if !style.generics.is_empty() {
                    unimplemented!("monomorphization of generic styles is not implemented yet")
                }
            }
        }
    }

    ///Resolves every generic struct/component reference inside `ty` to its
    ///specialization, recursively. Non-resolvable references are rebuilt with
    ///their sub-types resolved.
    fn resolve_expression_type(
        &mut self,
        hir: &SlynxHir,
        ty: DedupPoolId<HirType>,
        span: Span,
    ) -> Result<DedupPoolId<HirType>> {
        if is_resolvable_reference(hir, ty) {
            let ty_view = hir.view(ty);
            let deref = ty_view.dereference();
            return if deref.is_struct().is_some() {
                self.resolve_object_target(hir, ty, span)
            } else if deref.is_component().is_some() {
                self.resolve_component_target(hir, ty, span)
            } else {
                unreachable!("Resolvable references only target structs or components")
            };
        }

        match hir.view(ty).raw() {
            HirType::ImutableRef(inner) => Ok(hir.create_type(HirType::ImutableRef(
                self.resolve_expression_type(hir, *inner, span)?,
            ))),
            HirType::MutableRef(inner) => Ok(hir.create_type(HirType::MutableRef(
                self.resolve_expression_type(hir, *inner, span)?,
            ))),
            HirType::Array(inner, len) => Ok(hir.create_type(HirType::Array(
                self.resolve_expression_type(hir, *inner, span)?,
                *len,
            ))),
            HirType::Vector(inner) => Ok(hir.create_type(HirType::Vector(
                self.resolve_expression_type(hir, *inner, span)?,
            ))),
            HirType::Function(function) => {
                let function_view = hir.view(*function);
                let args = function_view
                    .arguments()
                    .iter()
                    .map(|arg| self.resolve_expression_type(hir, *arg, span))
                    .collect::<Result<Vec<_>>>()?;
                let ret = self.resolve_expression_type(hir, function_view.return_type(), span)?;
                Ok(hir.create_function_type(args, ret))
            }
            HirType::Tuple(tuple) => {
                let tuple_view = hir.view(*tuple);
                let fields = tuple_view
                    .fields()
                    .iter()
                    .map(|field| self.resolve_expression_type(hir, *field, span))
                    .collect::<Result<Vec<_>>>()?;
                Ok(hir.create_tuple_type(fields))
            }
            HirType::Component(component) => {
                self.rebuild_component_type(hir, *component, &Substitution::empty(), span)
            }
            HirType::Reference { rf, generics } => {
                let new_rf = self.resolve_expression_type(hir, *rf, span)?;
                let mut new_generics = *generics;
                for slot in &mut new_generics {
                    if !slot.is_null() {
                        *slot = self.resolve_expression_type(hir, *slot, span)?;
                    }
                }
                Ok(hir.create_type(HirType::Reference {
                    rf: new_rf,
                    generics: new_generics,
                }))
            }
            HirType::Nullable(inner) => {
                let new_inner = self.resolve_expression_type(hir, *inner, span)?;
                Ok(hir.create_type(HirType::Nullable(new_inner)))
            }
            other => Ok(hir.create_type(other.clone())),
        }
    }

    ///Records the type of a `let`-bound variable in the innermost active scope.
    fn declare_variable(&mut self, id: VariableId, tracked: TrackedVariable) {
        self.variable_types
            .last_mut()
            .expect("A variable declaration requires an active scope")
            .insert(id, tracked);
    }

    ///Looks up the type of a `let`-bound variable across the active scopes,
    ///innermost first. Returns `None` for function arguments and statics,
    ///whose types the substitution already resolves.
    fn lookup_variable_type(&self, id: VariableId) -> Option<TrackedVariable> {
        self.variable_types
            .iter()
            .rev()
            .find_map(|scope| scope.get(&id).copied())
    }

    ///Rebuilds a list of statements, substituting generic parameters and
    ///rewriting generic call sites and struct/component usage. Each call
    ///enters a fresh lexical scope so `let` bindings tracked inside a block do
    ///not leak out of it.
    fn build_statements(
        &mut self,
        hir: &SlynxHir,
        statements: &[Spanned<PoolId<HirStatement>>],
        subst: &Substitution,
    ) -> Result<Vec<Spanned<PoolId<HirStatement>>>> {
        self.variable_types.push(HashMap::new());
        let result = statements
            .iter()
            .map(|statement| self.build_statement(hir, *statement, subst))
            .collect();
        self.variable_types.pop();
        result
    }

    fn build_statement(
        &mut self,
        hir: &SlynxHir,
        statement: Spanned<PoolId<HirStatement>>,
        subst: &Substitution,
    ) -> Result<Spanned<PoolId<HirStatement>>> {
        let new_statement = match &hir[statement.data] {
            HirStatement::Assign { lhs, value } => HirStatement::Assign {
                lhs: self.build_expression(hir, *lhs, subst)?,
                value: self.build_expression(hir, *value, subst)?,
            },
            HirStatement::Variable { name, value } => {
                let original = hir[value.data].ty;
                let value = self.build_expression(hir, *value, subst)?;
                let rebuilt = hir[value.data].ty;
                self.declare_variable(*name, TrackedVariable { original, rebuilt });
                HirStatement::Variable { name: *name, value }
            }
            HirStatement::Expression { expr } => HirStatement::Expression {
                expr: self.build_expression(hir, *expr, subst)?,
            },
            HirStatement::Return { expr } => HirStatement::Return {
                expr: expr
                    .map(|expr| self.build_expression(hir, expr, subst))
                    .transpose()?,
            },
            HirStatement::While { condition, body } => HirStatement::While {
                condition: self.build_expression(hir, *condition, subst)?,
                body: self.build_statements(hir, body, subst)?,
            },
        };
        let id = hir.insert_statement(new_statement);
        Ok(statement.span.make_spanned(id))
    }

    fn build_expression(
        &mut self,
        hir: &SlynxHir,
        expression: Spanned<PoolId<HirExpression>>,
        subst: &Substitution,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        let node = &hir[expression.data];
        let mut call_ty = node.ty;

        let kind = match node.kind.clone() {
            HirExpressionKind::Null
            | HirExpressionKind::Int(_)
            | HirExpressionKind::StringLiteral(_)
            | HirExpressionKind::Float(_)
            | HirExpressionKind::True
            | HirExpressionKind::False
            | HirExpressionKind::Static { .. } => node.kind.clone(),
            HirExpressionKind::Reference(inner) => {
                HirExpressionKind::Reference(self.build_expression(hir, inner, subst)?)
            }
            HirExpressionKind::Deref(inner) => {
                HirExpressionKind::Deref(self.build_expression(hir, inner, subst)?)
            }
            HirExpressionKind::Identifier(id) => {
                if let Some(tracked) = self.lookup_variable_type(id) {
                    // No annotation: the identifier carries the initializer's
                    // original type, so use the concrete rebuilt type. With an
                    // annotation the identifier already carries the declared
                    // type, which the substitution below resolves.
                    call_ty = if tracked.original == node.ty {
                        tracked.rebuilt
                    } else {
                        node.ty
                    };
                }
                node.kind.clone()
            }
            HirExpressionKind::Tuple(items) => {
                HirExpressionKind::Tuple(self.build_expressions(hir, &items, subst)?)
            }
            HirExpressionKind::Array(items) => {
                HirExpressionKind::Array(self.build_expressions(hir, &items, subst)?)
            }
            HirExpressionKind::Vector(items) => {
                HirExpressionKind::Vector(self.build_expressions(hir, &items, subst)?)
            }
            HirExpressionKind::ArrayIndex(array, index) => {
                let array = self.build_expression(hir, array, subst)?;
                let index = self.build_expression(hir, index, subst)?;
                let array_ty = hir[array.data].ty;
                call_ty = hir
                    .view(array_ty)
                    .is_vector()
                    .or_else(|| hir.view(array_ty).is_array().map(|(inner, _)| inner))
                    .ok_or_else(|| {
                        slynx_hir::HIRError::invalid_indexing(array_ty, expression.span)
                    })?;
                HirExpressionKind::ArrayIndex(array, index)
            }
            HirExpressionKind::Binary { lhs, op, rhs } => HirExpressionKind::Binary {
                lhs: self.build_expression(hir, lhs, subst)?,
                op,
                rhs: self.build_expression(hir, rhs, subst)?,
            },
            HirExpressionKind::Object { name, fields } => {
                let substituted_name = substitute_type(hir, name, subst)?;
                let ty_view = hir.view(substituted_name);
                let deref = ty_view.dereference();
                let new_name = if is_resolvable_reference(hir, substituted_name)
                    && deref.is_struct().is_some()
                {
                    call_ty = self.resolve_object_target(hir, substituted_name, expression.span)?;
                    call_ty
                } else {
                    substituted_name
                };
                HirExpressionKind::Object {
                    name: new_name,
                    fields: self.build_expressions(hir, &fields, subst)?,
                }
            }
            HirExpressionKind::FieldAccess {
                expr,
                field_index,
                field_name,
            } => {
                let expr = self.build_expression(hir, expr, subst)?;
                let parent_ty = hir[expr.data].ty;
                call_ty = match hir.view(parent_ty).dereference().is_struct() {
                    Some(struct_view) => struct_view
                        .field_types()
                        .get(field_index)
                        .copied()
                        .unwrap_or(node.ty),
                    None => node.ty,
                };
                HirExpressionKind::FieldAccess {
                    expr,
                    field_index,
                    field_name,
                }
            }
            HirExpressionKind::Component(component) => {
                let new_component = self.build_component_expression(hir, component, subst)?;
                call_ty = hir[new_component.data].name;
                HirExpressionKind::Component(new_component)
            }
            HirExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => HirExpressionKind::If {
                condition: self.build_expression(hir, condition, subst)?,
                then_branch: self.build_statements(hir, &then_branch, subst)?,
                else_branch: else_branch
                    .map(|branch| self.build_statements(hir, &branch, subst))
                    .transpose()?,
            },
            HirExpressionKind::FunctionCall {
                name,
                args,
                generics,
            } => {
                let new_generics = generics
                    .iter()
                    .map(|generic| substitute_type(hir, *generic, subst))
                    .collect::<Result<Vec<_>>>()?;

                let new_name = if new_generics.is_empty() {
                    name
                } else {
                    let target =
                        self.resolve_function_target(hir, name, new_generics, expression.span)?;
                    let AnyLocalDeclarationId::Function(local_id) = target.local_id else {
                        unreachable!("A monomorphized call target must be a function")
                    };
                    call_ty = self.function_return_type(hir, target)?;
                    DeclarationId::new(target.file_id, local_id)
                };

                HirExpressionKind::FunctionCall {
                    name: new_name,
                    args: self.build_expressions(hir, &args, subst)?,
                    generics: Vec::new(),
                }
            }
        };

        let substituted_ty = substitute_type(hir, call_ty, subst)?;
        let ty = if contains_resolvable_reference(hir, substituted_ty) {
            self.resolve_expression_type(hir, substituted_ty, expression.span)?
        } else {
            substituted_ty
        };
        let id = hir.insert_expression(HirExpression { ty, kind });
        Ok(expression.span.make_spanned(id))
    }

    fn build_expressions(
        &mut self,
        hir: &SlynxHir,
        expressions: &[Spanned<PoolId<HirExpression>>],
        subst: &Substitution,
    ) -> Result<Vec<Spanned<PoolId<HirExpression>>>> {
        expressions
            .iter()
            .map(|expression| self.build_expression(hir, *expression, subst))
            .collect()
    }

    fn build_component_expression(
        &mut self,
        hir: &SlynxHir,
        component: Spanned<PoolId<HirComponentExpression>>,
        subst: &Substitution,
    ) -> Result<Spanned<PoolId<HirComponentExpression>>> {
        let node = &hir[component.data];
        let substituted_name = substitute_type(hir, node.name, subst)?;
        let ty_view = hir.view(substituted_name);
        let deref = ty_view.dereference();
        let new_name =
            if is_resolvable_reference(hir, substituted_name) && deref.is_component().is_some() {
                self.resolve_component_target(hir, substituted_name, component.span)?
            } else {
                substituted_name
            };
        let new_component = HirComponentExpression {
            name: new_name,
            properties: node
                .properties
                .iter()
                .map(|property| {
                    let expr = self.build_expression(hir, *property.expr(), subst)?;
                    Ok(PropertyExpression::new(property.index(), expr))
                })
                .collect::<Result<Vec<_>>>()?,
            children: node
                .children
                .iter()
                .map(|child| self.build_component_expression(hir, *child, subst))
                .collect::<Result<Vec<_>>>()?,
        };
        let id = hir.insert_component_expression(new_component);
        Ok(component.span.make_spanned(id))
    }
}
