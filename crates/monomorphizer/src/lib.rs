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
//! - [`functions`](self::functions) — function specialization.
//! - [`structs`](self::structs) — struct (object) specialization.
//! - [`components`](self::components) — component specialization.
//! - [`enums`](self::enums) — enum specialization.
//! - [`types`](self::types) — shared type-substitution machinery.
//! - [`specialization`](self::specialization) — the
//!   [`SpecializationDescriptor`](self::specialization::SpecializationDescriptor)
//!   request bundle and the declaration-kind traits `specialize` is generic
//!   over.
//!
//! See `docs/architecture.md` in this crate for the full picture, and
//! `docs/extension-guide.md` for how to add another declaration kind.
//!
//! The [`Monomorphizer`] struct owns the shared state (memoization cache,
//! in-progress set for cycle detection, and the dead-code set) and the
//! declaration-agnostic tree builders that every kind reuses. Everything that
//! is not shared state is generic over the declaration kind: the driver
//! [`Monomorphizer::specialize`] takes a
//! [`SpecializationDescriptor`](self::specialization::SpecializationDescriptor)
//! and works on typed `DeclarationId<K>`s, erasing to [`AnyDeclarationId`]
//! only at the two heterogeneous borders — the cache and the dead-code set.

mod components;
mod enums;
mod functions;
mod specialization;
mod structs;
mod types;

use std::collections::{HashMap, HashSet};

use common::{PoolStorage, Span, Spanned, pool::PoolId};
use dashmap::DashMap;
use module_loader::FileId;
use slynx_hir::{
    DeclarationId, DeclarationsPool, DescriptorId, HIRError, HirComponentExpression, HirExpression,
    HirExpressionKind, HirFunctionDeclaration, HirStatement, PropertyExpression, Result, SlynxHir,
    SymbolPointer, TypeDeclaration, VariableId,
    id::AnyDeclarationId,
    term::{Term, TermId, TermNode},
};

use specialization::{SpecializableDeclaration, SpecializationDescriptor};
use types::{
    MonomorphizationKey, Substitution, contains_resolvable_reference, is_resolvable_reference,
    mangle_name, substitute_type,
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
    original: TermId,
    rebuilt: TermId,
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
    ///
    /// This is one of the two borders where this pass speaks the universal
    /// [`AnyDeclarationId`]: specializations of every declaration kind share
    /// one map, so a hit comes back erased and is narrowed back to a typed id
    /// through [`SpecializableDeclaration::from_erased`]. The companion
    /// [`Monomorphizer::in_progress`] set inherits the same key type.
    cache: DashMap<MonomorphizationKey, AnyDeclarationId>,
    /// Instantiations currently being generated. Used to detect
    /// non-terminating instantiations (a key that re-enters itself while still
    /// in progress).
    in_progress: HashSet<MonomorphizationKey>,
    /// The generic templates that were neutralized and are now dead code.
    ///
    /// The other heterogeneous border of this pass: the set mixes every
    /// declaration kind so codegen can skip dead declarations with a single
    /// lookup, and it is what [`Monomorphizer::resolve`] returns. Templates
    /// enter it through [`Monomorphizer::mark_dead`], which erases their typed
    /// id with [`TypeDeclaration::as_any_id`].
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
    /// Generic type aliases and stylesheets are currently unsupported and
    /// produce a diagnostic error.
    ///
    /// Returns the set of generic templates that were neutralized and should
    /// be treated as dead code. The set is heterogeneous ([`AnyDeclarationId`])
    /// because codegen skips dead declarations of every kind through a single
    /// lookup.
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

    /// Drives the pass over every file of `hir` in four steps.
    ///
    /// See the step comments below (and `docs/architecture.md`) for the order:
    /// rewrite function bodies → rewrite component members → resolve generic
    /// references in signatures → neutralize the remaining generic templates.
    ///
    /// # Arguments
    ///
    /// * `hir` — the finished HIR to monomorphize. Bodies, signatures, and
    ///   specializations are written in place through the store's interior
    ///   mutability, so a shared borrow suffices here.
    fn run(&mut self, hir: &SlynxHir) -> Result<()> {
        self.assert_no_generic_non_functions(hir)?;

        let files: Vec<FileId> = hir.store.files.iter().map(|file| *file.key()).collect();

        // Step 1: rewrite every non-generic function body, resolving generic
        // call sites and generic struct/component usage as they are found.
        // Specializations may discover further generic usage and instantiate it
        // recursively.
        for owner in &files {
            let targets: Vec<FunctionSnapshot> = {
                let file = hir.get_file(*owner);
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
                let mut file = hir.get_file_mut(*owner);
                file.declarations
                    .declarations
                    .functions
                    .get_mut(local_id)
                    .statements = new_statements;
            }
        }

        // Step 2: rewrite the members (property defaults and child tree) of
        // every non-generic component.
        for owner in &files {
            let ids: Vec<PoolId<slynx_hir::HirComponentDeclaration>> = {
                let file = hir.get_file(*owner);
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
                self.rewrite_non_generic_component(hir, *owner, local_id)?;
            }
        }

        // Step 3: resolve generic struct/component references in the signatures
        // of non-generic functions and components.
        for owner in &files {
            let function_ids: Vec<PoolId<HirFunctionDeclaration>> = {
                let file = hir.get_file(*owner);
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
                let old_ty = hir.get_file(*owner).declarations.declarations.functions[local_id].ty;
                if contains_resolvable_reference(hir, old_ty) {
                    let new_ty = self.resolve_expression_type(hir, old_ty, Span::default())?;
                    hir.get_file_mut(*owner)
                        .declarations
                        .declarations
                        .functions
                        .get_mut(local_id)
                        .ty = new_ty;
                }
            }

            let component_ids: Vec<PoolId<slynx_hir::HirComponentDeclaration>> = {
                let file = hir.get_file(*owner);
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
                let old_ty = hir.get_file(*owner).declarations.declarations.components[local_id].ty;
                if contains_resolvable_reference(hir, old_ty) {
                    let new_ty = self.resolve_expression_type(hir, old_ty, Span::default())?;
                    hir.get_file_mut(*owner)
                        .declarations
                        .declarations
                        .components
                        .get_mut(local_id)
                        .ty = new_ty;
                }
            }
        }

        // Step 4: neutralize every generic template so codegen never sees a
        // `GenericParam`-typed signature, and mark each one as dead. Each
        // kind owns its neutralization in its own module; they all share the
        // `generic_templates` scan and the `mark_dead` bookkeeping below.
        let void_ty = hir
            .types
            .create_function_type(Vec::new(), hir.types.create_type(Term::void_type()));
        self.neutralize_generic_functions(hir, &files, void_ty);
        self.neutralize_generic_objects(hir, &files, void_ty);
        self.neutralize_generic_components(hir, &files, void_ty);
        self.neutralize_generic_enums(hir, &files, void_ty);

        Ok(())
    }

    /// Specializes `request.template` for `request.args`, generating the
    /// concrete copy on first use and returning its typed id.
    ///
    /// This is the one shared skeleton behind every `resolve_*_target` in the
    /// per-kind modules. The recipe:
    ///
    /// 1. **Arity check** — the template's generic parameter count must equal
    ///    the number of requested type arguments, else
    ///    [`HIRError::generic_arity_mismatch`].
    /// 2. **Cache hit** — if `(template, args)` was already specialized, the
    ///    erased id stored in the cache is narrowed back through
    ///    [`SpecializableDeclaration::from_erased`] and returned. This is the
    ///    only place this pass undoes an erasure: the cache is heterogeneous
    ///    by design (see [`Monomorphizer::cache`]).
    /// 3. **Cycle check** — re-entering an in-progress key means a
    ///    non-terminating instantiation, reported as
    ///    [`HIRError::cyclic_monomorphization`].
    /// 4. **Build** — the descriptor's `build` callback inserts the
    ///    specialization next to the template under a mangled name and returns
    ///    its id; the id is then memoized in the cache and the template is
    ///    marked dead (a concrete copy now exists for it).
    ///
    /// # Arguments
    ///
    /// * `hir` — the HIR being monomorphized; the specialization is inserted
    ///   into the template's own file so codegen hoists it alongside its
    ///   template.
    /// * `request` — the instantiation request; see
    ///   [`SpecializationDescriptor`] for its fields.
    ///
    /// # Type parameters
    ///
    /// * `K` — the declaration kind being specialized.
    /// * `B` — the kind-specific build callback the request carries.
    fn specialize<K, B>(
        &mut self,
        hir: &SlynxHir,
        request: SpecializationDescriptor<K, B>,
    ) -> Result<DeclarationId<K>>
    where
        K: SpecializableDeclaration,
        DeclarationsPool: PoolStorage<K>,
        B: FnOnce(
            &mut Monomorphizer,
            &SlynxHir,
            &Substitution,
            SymbolPointer,
            &MonomorphizationKey,
        ) -> Result<DeclarationId<K>>,
    {
        let SpecializationDescriptor {
            template,
            args,
            span,
            build,
        } = request;

        // The template's own name and arity drive the diagnostics and the
        // mangled specialization name; both are read generically through the
        // kind's storage column.
        let (name, generic_count) = {
            let file = hir.get_file(template.owner);
            let pool = <DeclarationsPool as PoolStorage<K>>::get_pool(&file.declarations);
            let declaration = &pool[template.term];
            (declaration.name(), declaration.generics().len())
        };

        if generic_count != args.len() {
            return Err(HIRError::generic_arity_mismatch(
                name,
                generic_count,
                args.len(),
                span,
            ));
        }

        let template_erased = K::as_any_id(template);
        let key: MonomorphizationKey = (template_erased, args.clone().into());
        if let Some(cached) = self.cache.get(&key) {
            let specialized = K::from_erased(*cached)
                .expect("a cache hit for a template can only name the same declaration kind");
            return Ok(specialized);
        }
        if self.in_progress.contains(&key) {
            return Err(HIRError::cyclic_monomorphization(name, args, span));
        }
        self.in_progress.insert(key.clone());

        let subst = Substitution::new(&args);
        let mangled_symbol = hir.intern_name(&mangle_name(hir, name, &args));
        let specialized = build(self, hir, &subst, mangled_symbol, &key)?;

        self.in_progress.remove(&key);
        self.cache.insert(key, K::as_any_id(specialized));
        self.dead_code.insert(template_erased);

        Ok(specialized)
    }

    /// Finds the declaration of kind `D` named `name` in any file of `hir`.
    ///
    /// The declaration's name is read through [`NamedDeclaration`] and its
    /// column through [`PoolStorage`], so the lookup is a plain
    /// linear scan with no per-kind selector to keep in sync.
    ///
    /// # Arguments
    ///
    /// * `hir` — the HIR whose per-file declaration pools are scanned.
    /// * `name` — the declared name to look for.
    ///
    /// # Returns
    ///
    /// The typed id of the first declaration whose name equals `name`,
    /// scanning files in store order.
    fn find_declaration_by_name<D>(
        &self,
        hir: &SlynxHir,
        name: SymbolPointer,
    ) -> Option<DeclarationId<D>>
    where
        D: TypeDeclaration,
        DeclarationsPool: PoolStorage<D>,
    {
        for file in hir.store.files.iter() {
            let pool = <DeclarationsPool as PoolStorage<D>>::get_pool(&file.declarations);
            for (local_id, declaration) in pool.iter().with_ids() {
                if declaration.name() == name {
                    return Some(DeclarationId::new(file.file, local_id));
                }
            }
        }
        None
    }

    /// Collects every generic template of declaration kind `D` across `files`.
    ///
    /// A "generic template" is a declaration whose generic parameter list is
    /// not empty — one that either still needs to be specialized at a use site
    /// or must be neutralized before codegen runs.
    ///
    /// This is the shared scan behind the per-kind `neutralize_generic_*`
    /// methods; it returns typed ids so callers never touch an
    /// [`AnyDeclarationId`] until they hand one to
    /// [`Monomorphizer::mark_dead`].
    ///
    /// # Arguments
    ///
    /// * `hir` — the HIR whose files are scanned.
    /// * `files` — the files to scan.
    ///
    /// # Returns
    ///
    /// The typed ids of the matching templates, in file then pool order.
    fn generic_templates<D>(&self, hir: &SlynxHir, files: &[FileId]) -> Vec<DeclarationId<D>>
    where
        D: TypeDeclaration,
        DeclarationsPool: PoolStorage<D>,
    {
        let mut templates = Vec::new();
        for owner in files {
            let file = hir.get_file(*owner);
            let pool = <DeclarationsPool as PoolStorage<D>>::get_pool(&file.declarations);
            templates.extend(
                pool.iter()
                    .with_ids()
                    .filter(|(_, declaration)| !declaration.generics().is_empty())
                    .map(|(local_id, _)| DeclarationId::new(*owner, local_id)),
            );
        }
        templates
    }

    /// Records a neutralized template in the dead-code set.
    ///
    /// The set is heterogeneous ([`AnyDeclarationId`]) because codegen skips
    /// dead declarations of every kind through one lookup; erasing the typed
    /// id through [`TypeDeclaration::as_any_id`] keeps that border explicit
    /// and greppable.
    fn mark_dead<D: TypeDeclaration>(&mut self, template: DeclarationId<D>) {
        self.dead_code.insert(D::as_any_id(template));
    }

    ///Monomorphization of generic type aliases and stylesheets is not supported
    ///yet, so encountering one is a hard error.
    fn assert_no_generic_non_functions(&self, hir: &SlynxHir) -> Result<()> {
        for file in hir.store.files.iter() {
            for alias in file.declarations.declarations.alias.iter() {
                if !alias.generics.is_empty() {
                    return Err(HIRError::not_implemented(alias.name, Span::default()));
                }
            }
        }
        Ok(())
    }

    ///Resolves every generic struct/component reference inside `ty` to its
    ///specialization, recursively. Non-resolvable references are rebuilt with
    ///their sub-types resolved.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose type pool receives the rebuilt types.
    ///* `ty` — the type to walk.
    ///* `span` — the use-site span, forwarded to the per-kind
    ///  `resolve_*_target` specializations for diagnostics.
    fn resolve_expression_type(
        &mut self,
        hir: &SlynxHir,
        ty: TermId,
        span: Span,
    ) -> Result<TermId> {
        if is_resolvable_reference(hir, ty) {
            let ty_view = hir.view(ty);
            let deref = ty_view.nominal();
            return if deref.is_struct().is_some() {
                self.resolve_object_target(hir, ty, span)
            } else if deref.is_component().is_some() {
                self.resolve_component_target(hir, ty, span)
            } else if deref.is_enum().is_some() {
                self.resolve_enum_target(hir, ty, span)
            } else {
                unreachable!(
                    "Resolvable references only target structs, components, or enums. Type: '{:?}' '{}'",
                    deref.data(),
                    deref.pretty_name()
                )
            };
        }

        match hir.view(ty).raw().node() {
            TermNode::Ref { mutable, target } => Ok(hir.types.create_type({
                let expr_ty = self.resolve_expression_type(hir, *target, span)?;
                if *mutable {
                    Term::mutable_reference(expr_ty)
                } else {
                    Term::reference(expr_ty)
                }
            })),
            TermNode::Extension(ext) => Ok(hir.types.create_type(Term::extension(
                ext.try_map_children(&mut |id| self.resolve_expression_type(hir, id, span))?,
            ))),

            TermNode::Func { args, ret } => {
                let args = args
                    .iter()
                    .map(|arg| self.resolve_expression_type(hir, *arg, span))
                    .collect::<Result<Vec<_>>>()?;
                let ret = self.resolve_expression_type(hir, *ret, span)?;
                Ok(hir.types.create_function_type(args, ret))
            }
            TermNode::Tuple { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| self.resolve_expression_type(hir, *field, span))
                    .collect::<Result<Vec<_>>>()?;
                Ok(hir.types.create_tuple_type(fields))
            }
            TermNode::Data(DescriptorId::Component(component)) => {
                self.rebuild_component_type(hir, *component, &Substitution::empty(), span)
            }
            TermNode::Apply { target, args } => {
                let new_rf = self.resolve_expression_type(hir, *target, span)?;
                let mut new_generics = args.clone();
                for slot in &mut new_generics {
                    if !slot.is_null() {
                        *slot = self.resolve_expression_type(hir, *slot, span)?;
                    }
                }
                Ok(hir
                    .types
                    .create_type(Term::application(new_rf, new_generics)))
            }

            _ => Ok(ty),
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
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR; rebuilt statements and expressions are inserted into
    ///  its pools.
    ///* `statements` — the statements to rebuild.
    ///* `subst` — the substitution of generic parameters for this
    ///  instantiation.
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

    ///Rebuilds a single statement under `subst`, inserting the rebuilt copy
    ///into the statement pool and returning it under the original span.
    ///
    ///`let`-bound variables are recorded with both their pre-substitution and
    ///rebuilt types (see [`TrackedVariable`]) so identifiers referencing them
    ///later can pick up the concrete type.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose statement pool receives the rebuilt copy.
    ///* `statement` — the statement to rebuild.
    ///* `subst` — the substitution of generic parameters for this
    ///  instantiation.
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
                let original = hir[value.data].ty.term;
                let value = self.build_expression(hir, *value, subst)?;
                let rebuilt = hir[value.data].ty.term;
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
        let id = hir.store.insert_statement(new_statement);
        Ok(statement.span.make_spanned(id))
    }

    ///Rebuilds a single expression under `subst`, inserting the rebuilt copy
    ///into the expression pool and returning it under the original span.
    ///
    ///This is where the declaration-kind triggers fire: a generic call site
    ///(`identity<int>(x)`) is specialized through
    ///[`Monomorphizer::resolve_function_target`], an object literal over a
    ///generic struct through [`Monomorphizer::resolve_object_target`], and a
    ///component expression through
    ///[`Monomorphizer::resolve_component_target`]. The expression's type is
    ///substituted (and, when it still references a resolvable specialization,
    ///resolved) alongside the kind rewrite.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose expression pool receives the rebuilt copy.
    ///* `expression` — the expression to rebuild.
    ///* `subst` — the substitution of generic parameters for this
    ///  instantiation.
    fn build_expression(
        &mut self,
        hir: &SlynxHir,
        expression: Spanned<PoolId<HirExpression>>,
        subst: &Substitution,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        let node = &hir[expression.data];
        let mut call_ty = node.ty.term;

        let kind = match node.kind.clone() {
            HirExpressionKind::Int(_)
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
                    call_ty = if tracked.original == node.ty.term {
                        tracked.rebuilt
                    } else {
                        node.ty.term
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
                    .view(array_ty.term)
                    .is_vector()
                    .or_else(|| hir.view(array_ty.term).is_array().map(|(inner, _)| inner))
                    .ok_or_else(|| {
                        slynx_hir::HIRError::invalid_indexing(array_ty.term, expression.span)
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
                let deref = ty_view.nominal();
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
                call_ty = match hir.view(parent_ty.term).dereference().is_struct() {
                    Some(struct_view) => struct_view
                        .fields()
                        .get(field_index)
                        .map(|field| field.ty)
                        .unwrap_or(node.ty.term),
                    None => node.ty.term,
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
            HirExpressionKind::Enum { ty, variant, args } => HirExpressionKind::Enum {
                ty: substitute_type(hir, ty, subst)?,
                variant,
                args: self.build_expressions(hir, &args, subst)?,
            },
            HirExpressionKind::Matches {
                value,
                variant,
                args,
            } => HirExpressionKind::Matches {
                value: self.build_expression(hir, value, subst)?,
                variant,
                args: self.build_expressions(hir, &args, subst)?,
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

                let new_name = if let Some(signature) = hir.types.interface_signature(name) {
                    // A deferred interface call: the target was the interface
                    // method's signature declaration because the receiver was a
                    // generic parameter. Now that its concrete type is known,
                    // pick the implementation that extends it.
                    let target = self.resolve_interface_call(
                        hir,
                        &signature,
                        args.first()
                            .map(|receiver| hir[receiver.data].ty.term)
                            .ok_or_else(|| {
                                HIRError::unresolved_interface_call(
                                    signature.name,
                                    node.ty.term,
                                    expression.span,
                                )
                            })?,
                        subst,
                        expression.span,
                    )?;
                    call_ty = self.function_return_type(hir, target)?;
                    target
                } else if new_generics.is_empty() {
                    name
                } else {
                    let target =
                        self.resolve_function_target(hir, name, new_generics, expression.span)?;
                    call_ty = self.function_return_type(hir, target)?;
                    target
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
        let id = hir.store.insert_expression(HirExpression {
            ty: slynx_hir::Owned::new(node.ty.owner, ty),
            kind,
        });
        Ok(expression.span.make_spanned(id))
    }

    ///Rebuilds a list of expressions one by one under the same `subst`,
    ///preserving order (see [`Monomorphizer::build_statement`]).
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

    ///Rebuilds a component expression (its name type, property values, and
    ///child tree) under `subst`.
    ///
    ///When the component name is a resolvable reference to a generic
    ///component, it is specialized through
    ///[`Monomorphizer::resolve_component_target`] first, so the rebuilt
    ///expression points at the concrete component type.
    ///
    ///# Arguments
    ///
    ///* `hir` — the HIR whose component-expression pool receives the rebuilt
    ///  copy.
    ///* `component` — the component expression to rebuild.
    ///* `subst` — the substitution of generic parameters for this
    ///  instantiation.
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
        let id = hir.store.insert_component_expression(new_component);
        Ok(component.span.make_spanned(id))
    }
}
