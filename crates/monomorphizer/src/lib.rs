use std::collections::{HashMap, HashSet};

use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use dashmap::DashMap;
use module_loader::FileId;
use slynx_hir::{
    DeclarationId, HIRError, HirComponentExpression, HirExpression, HirExpressionKind,
    HirFunctionDeclaration, HirStatement, HirType, PropertyExpression, Result, SlynxHir,
    SymbolPointer,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};
use smallvec::SmallVec;

/// Maps a generic parameter index to the concrete type it should be
/// substituted with.
type Substitution = HashMap<u8, DedupPoolId<HirType>>;

/// The key of a monomorphization: the generic template declaration together
/// with the concrete type arguments supplied at a call site.
type MonomorphizationKey = (AnyDeclarationId, SmallVec<[DedupPoolId<HirType>; 2]>);

/// A snapshot of a function to be rewritten: its local declaration id and the
/// ids of its statements.
type FunctionSnapshot = (
    PoolId<HirFunctionDeclaration>,
    Vec<Spanned<PoolId<HirStatement>>>,
);

/// A struct that handles all the monomorphization on the code.
///
/// Monomorphization specializes generic functions by instantiating them with
/// the concrete type arguments requested at call sites. Every call to a
/// generic function with explicit type arguments is rewritten to point at a
/// freshly generated, concrete (non-generic) copy of the function body.
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
}

impl Monomorphizer {
    /// Monomorphizes every generic function of the given `hir`.
    ///
    /// Generic non-function declarations (objects, components, styles, aliases)
    /// are currently unsupported and will `unimplemented!()`.
    ///
    /// Returns the set of generic templates that were neutralized and should
    /// be treated as dead code.
    pub fn resolve(hir: &mut SlynxHir) -> Result<HashSet<AnyDeclarationId>> {
        let mut monomorphizer = Self {
            cache: DashMap::new(),
            in_progress: HashSet::new(),
            dead_code: HashSet::new(),
        };
        monomorphizer.run(hir)?;
        Ok(monomorphizer.dead_code)
    }

    fn run(&mut self, hir: &SlynxHir) -> Result<()> {
        self.assert_no_generic_non_functions(hir);

        let files: Vec<FileId> = hir.files.iter().map(|file| *file.key()).collect();

        // Rewrite every non-generic function body, resolving generic call
        // sites as they are found. This is the driver of all instantiations:
        // specializations may discover further generic calls and instantiate
        // them recursively.
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
                let new_statements = self.build_statements(hir, &statements, &HashMap::new())?;
                let mut file = hir.get_file_mut(*file_id);
                file.declarations
                    .declarations
                    .functions
                    .get_mut(local_id)
                    .statements = new_statements;
            }
        }

        // Neutralize every generic template so codegen never sees a
        // `GenericParam`-typed signature, and mark it as dead.
        let void_ty = hir.create_function_type(Vec::new(), hir.create_type(HirType::Void));
        for file_id in &files {
            let generic_ids: Vec<PoolId<HirFunctionDeclaration>> = {
                let file = hir.get_file(*file_id);
                file.declarations
                    .declarations
                    .functions
                    .iter()
                    .with_ids()
                    .filter(|(_, declaration)| !declaration.generics.is_empty())
                    .map(|(id, _)| id)
                    .collect()
            };

            for local_id in generic_ids {
                let mut file = hir.get_file_mut(*file_id);
                let declaration = file.declarations.declarations.functions.get_mut(local_id);
                declaration.statements = Vec::new();
                declaration.ty = void_ty;
                self.dead_code.insert(AnyDeclarationId::new(
                    *file_id,
                    AnyLocalDeclarationId::Function(local_id),
                ));
            }
        }

        Ok(())
    }

    ///Monomorphization of generic non-function declarations is not supported
    ///yet, so encountering one is a hard error (`unimplemented!()`).
    fn assert_no_generic_non_functions(&self, hir: &SlynxHir) {
        for file in hir.files.iter() {
            for object in file.declarations.declarations.objects.iter() {
                if !object.generics.is_empty() {
                    unimplemented!("monomorphization of generic objects is not implemented yet")
                }
            }
            for component in file.declarations.declarations.components.iter() {
                if !component.generics.is_empty() {
                    unimplemented!("monomorphization of generic components is not implemented yet")
                }
            }
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

    ///Generates (or retrieves from the cache) the specialization of `template`
    ///with the given concrete `args`.
    fn resolve_call_target(
        &mut self,
        hir: &SlynxHir,
        template: DeclarationId<HirFunctionDeclaration>,
        args: Vec<DedupPoolId<HirType>>,
        span: Span,
    ) -> Result<AnyDeclarationId> {
        let template_any = AnyDeclarationId::new(
            template.file_id,
            AnyLocalDeclarationId::Function(template.local_id),
        );

        let (name, generics, fargs, fty, statements, visibility, external, attributes) = {
            let file = hir.get_file(template.file_id);
            let declaration = &file.declarations.declarations.functions[template.local_id];
            (
                declaration.name,
                declaration.generics.clone(),
                declaration.args.clone(),
                declaration.ty,
                declaration.statements.clone(),
                declaration.visibility,
                declaration.external,
                declaration.attributes.clone(),
            )
        };

        if generics.len() != args.len() {
            return Err(HIRError::generic_arity_mismatch(
                name,
                generics.len(),
                args.len(),
                span,
            ));
        }

        let key: MonomorphizationKey = (template_any, args.clone().into());
        if let Some(cached) = self.cache.get(&key) {
            return Ok(*cached);
        }
        if self.in_progress.contains(&key) {
            return Err(HIRError::cyclic_monomorphization(name, args, span));
        }
        self.in_progress.insert(key.clone());

        let mut subst = HashMap::new();
        for (index, arg) in args.iter().enumerate() {
            subst.insert(index as u8, *arg);
        }

        let specialized_ty = self.substitute_type(hir, fty, &subst)?;
        let mangled_name = self.mangle_name(hir, name, &args);
        let mangled_symbol = hir.intern_name(&mangled_name);

        let specialized_local = {
            let file = hir.get_file_mut(template.file_id);
            file.declarations
                .declarations
                .functions
                .insert(HirFunctionDeclaration {
                    name: mangled_symbol,
                    generics: Vec::new(),
                    args: fargs,
                    ty: specialized_ty,
                    statements: Vec::new(),
                    visibility,
                    external,
                    attributes,
                })
        };
        let specialized = AnyDeclarationId::new(
            template.file_id,
            AnyLocalDeclarationId::Function(specialized_local),
        );

        // Cache *before* generating the body so recursive instantiations of the
        // same (template, args) resolve to this very declaration.
        self.cache.insert(key.clone(), specialized);

        let new_statements = self.build_statements(hir, &statements, &subst)?;
        {
            let mut file = hir.get_file_mut(template.file_id);
            file.declarations
                .declarations
                .functions
                .get_mut(specialized_local)
                .statements = new_statements;
        }

        self.in_progress.remove(&key);
        self.dead_code.insert(template_any);

        Ok(specialized)
    }

    ///Retrieves the return type of the given (already specialized) function
    ///declaration.
    fn function_return_type(
        &self,
        hir: &SlynxHir,
        id: AnyDeclarationId,
    ) -> Result<DedupPoolId<HirType>> {
        let AnyLocalDeclarationId::Function(local_id) = id.local_id else {
            unreachable!("A monomorphized call target must be a function")
        };
        let file = hir.get_file(id.file_id);
        let declaration = &file.declarations.declarations.functions[local_id];
        let view = hir.view(declaration.ty);
        let function = view
            .is_function()
            .expect("Function declaration should have a function type");
        Ok(function.return_type())
    }

    ///Computes the mangled name of a specialization: the template name
    ///followed by one `$<hash>` per concrete type argument, where the hash is
    ///computed structurally over the `HirType` value.
    fn mangle_name(
        &self,
        hir: &SlynxHir,
        name: SymbolPointer,
        args: &[DedupPoolId<HirType>],
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let base = hir.get_name(name);
        let mut out = String::with_capacity(base.len() + args.len() * 17);
        out.push_str(base);
        for arg in args {
            let mut hasher = DefaultHasher::new();
            hir.view(*arg).raw().hash(&mut hasher);
            out.push('_');
            out.push_str(&hir.view(*arg).name());
            out.push_str(&format!("_{:04x}", hasher.finish() & 0xffff));
        }
        out
    }

    ///Substitutes every generic parameter inside `ty` with its concrete type.
    fn substitute_type(
        &self,
        hir: &SlynxHir,
        ty: DedupPoolId<HirType>,
        subst: &Substitution,
    ) -> Result<DedupPoolId<HirType>> {
        match hir.view(ty).raw() {
            HirType::GenericParam { index, .. } => Ok(subst.get(index).copied().unwrap_or(ty)),
            HirType::Array(inner, len) => Ok(hir.create_type(HirType::Array(
                self.substitute_type(hir, *inner, subst)?,
                *len,
            ))),
            HirType::Vector(inner) => {
                Ok(hir.create_type(HirType::Vector(self.substitute_type(hir, *inner, subst)?)))
            }
            HirType::Function(function) => {
                let function_view = hir.view(*function);
                let args = function_view
                    .arguments()
                    .iter()
                    .map(|arg| self.substitute_type(hir, *arg, subst))
                    .collect::<Result<Vec<_>>>()?;
                let ret = self.substitute_type(hir, function_view.return_type(), subst)?;
                Ok(hir.create_function_type(args, ret))
            }
            HirType::Tuple(tuple) => {
                let tuple_view = hir.view(*tuple);
                let fields = tuple_view
                    .fields()
                    .iter()
                    .map(|field| self.substitute_type(hir, *field, subst))
                    .collect::<Result<Vec<_>>>()?;
                Ok(hir.create_tuple_type(fields))
            }
            HirType::Reference { rf, generics } => {
                let new_rf = self.substitute_type(hir, *rf, subst)?;
                let mut new_generics = *generics;
                for slot in &mut new_generics {
                    if let HirType::GenericParam { index, .. } = hir.view(*slot).raw()
                        && subst.contains_key(index)
                    {
                        *slot = self.substitute_type(hir, *slot, subst)?;
                    }
                }
                Ok(hir.create_type(HirType::Reference {
                    rf: new_rf,
                    generics: new_generics,
                }))
            }
            other => Ok(hir.create_type(other.clone())),
        }
    }

    ///Rebuilds a list of statements, substituting generic parameters and
    ///rewriting generic call sites.
    fn build_statements(
        &mut self,
        hir: &SlynxHir,
        statements: &[Spanned<PoolId<HirStatement>>],
        subst: &Substitution,
    ) -> Result<Vec<Spanned<PoolId<HirStatement>>>> {
        statements
            .iter()
            .map(|statement| self.build_statement(hir, *statement, subst))
            .collect()
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
            HirStatement::Variable { name, value } => HirStatement::Variable {
                name: *name,
                value: self.build_expression(hir, *value, subst)?,
            },
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
            HirExpressionKind::Int(_)
            | HirExpressionKind::StringLiteral(_)
            | HirExpressionKind::Float(_)
            | HirExpressionKind::True
            | HirExpressionKind::False
            | HirExpressionKind::Identifier(_)
            | HirExpressionKind::Static { .. } => node.kind.clone(),
            HirExpressionKind::Tuple(items) => {
                HirExpressionKind::Tuple(self.build_expressions(hir, &items, subst)?)
            }
            HirExpressionKind::Array(items) => {
                HirExpressionKind::Array(self.build_expressions(hir, &items, subst)?)
            }
            HirExpressionKind::Vector(items) => {
                HirExpressionKind::Vector(self.build_expressions(hir, &items, subst)?)
            }
            HirExpressionKind::ArrayIndex(array, index) => HirExpressionKind::ArrayIndex(
                self.build_expression(hir, array, subst)?,
                self.build_expression(hir, index, subst)?,
            ),
            HirExpressionKind::Binary { lhs, op, rhs } => HirExpressionKind::Binary {
                lhs: self.build_expression(hir, lhs, subst)?,
                op,
                rhs: self.build_expression(hir, rhs, subst)?,
            },
            HirExpressionKind::Object { name, fields } => HirExpressionKind::Object {
                name: self.substitute_type(hir, name, subst)?,
                fields: self.build_expressions(hir, &fields, subst)?,
            },
            HirExpressionKind::FieldAccess {
                expr,
                field_index,
                field_name,
            } => HirExpressionKind::FieldAccess {
                expr: self.build_expression(hir, expr, subst)?,
                field_index,
                field_name,
            },
            HirExpressionKind::Component(component) => HirExpressionKind::Component(
                self.build_component_expression(hir, component, subst)?,
            ),
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
                    .map(|generic| self.substitute_type(hir, *generic, subst))
                    .collect::<Result<Vec<_>>>()?;

                let new_name = if new_generics.is_empty() {
                    name
                } else {
                    let target =
                        self.resolve_call_target(hir, name, new_generics, expression.span)?;
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

        let ty = self.substitute_type(hir, call_ty, subst)?;
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
        let new_component = HirComponentExpression {
            name: self.substitute_type(hir, node.name, subst)?,
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
