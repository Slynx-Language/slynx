use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

use common::{
    Span, Spanned, VisibilityModifier,
    pool::{DedupPoolId, PoolId},
};
use module_loader::{ASTType, ASTTypeKind, FileId};
use slynx_parser::{
    ASTExpression, ASTStatement, ComponentExpression, ComponentMemberValue, RangeType, TypeContext,
};

use crate::{
    DeclarationId, HIRError, HirComponentExpression, HirExpression, HirExpressionKind,
    HirFunctionDeclaration, HirStatement, HirStaticDeclaration, HirType, PropertyExpression,
    Result, SymbolPointer, VariableId, builders::HirQueueBuilder, context::HirSymbol,
    helpers::Visible, id::OwnerId,
};

/// Result of building a body with the ExpressionBuilder.
pub(crate) struct ExpressionBuildResult {
    pub(crate) args: Vec<VariableId>,
    pub(crate) statements: Vec<Spanned<PoolId<HirStatement>>>,
}

pub(crate) enum HirName {
    Variable(VariableId),
    Static(DeclarationId<HirStaticDeclaration>),
}

/// A single, reusable expression builder that can be used by both function
/// builders and component builders. Owns the state required for expression
/// generation (variables, type mappings, etc.).
pub(crate) struct ExpressionBuilder {
    pub(crate) target: OwnerId,
    pub(crate) names: HashMap<SymbolPointer, VariableId>,
    pub(crate) variables_types: HashMap<VariableId, DedupPoolId<HirType>>,
    pub(crate) mutable: HashSet<VariableId>,
}

impl ExpressionBuilder {
    pub fn new(owner: OwnerId) -> Self {
        Self {
            target: owner,
            names: HashMap::new(),
            variables_types: HashMap::new(),
            mutable: HashSet::new(),
        }
    }

    pub fn file(&self) -> FileId {
        match self.target {
            OwnerId::Component(c) => c.file_id,
            OwnerId::Function(f) => f.file_id,
        }
    }

    pub fn create_mapped_variable(
        &mut self,
        name: SymbolPointer,
        id: VariableId,
        mutable: bool,
        ty: DedupPoolId<HirType>,
    ) {
        self.names.insert(name, id);
        self.variables_types.insert(id, ty);
        if mutable {
            self.mutable.insert(id);
        }
    }

    pub fn create_variable(
        &mut self,
        name: SymbolPointer,
        mutable: bool,
        ty: DedupPoolId<HirType>,
    ) -> VariableId {
        let id = VariableId::new(self.target, self.names.len() as u16);
        self.create_mapped_variable(name, id, mutable, ty);
        id
    }

    fn is_mutable(&self, id: VariableId) -> bool {
        self.mutable.contains(&id)
    }

    /// Resolves a name to either a local variable or a static declaration.
    /// Component method builders can extend this by resolving component fields
    /// before falling back to statics.
    pub(crate) fn resolve_name(
        &self,
        queue: &HirQueueBuilder,
        ptr: SymbolPointer,
        span: Span,
    ) -> Result<HirName> {
        if let Some(var) = self.names.get(&ptr).cloned() {
            Ok(HirName::Variable(var))
        } else if let Some((file_owner, statik)) = queue.find_static_declaration(ptr, self.file()) {
            let id = queue.enqueue_static(statik, queue.get_node(file_owner))?;
            Ok(HirName::Static(id))
        } else {
            Err(HIRError::name_unrecognized(ptr, span))
        }
    }

    fn unify_types(
        &self,
        queue: &HirQueueBuilder,
        received: DedupPoolId<HirType>,
        expected: DedupPoolId<HirType>,
        span: Span,
    ) -> Result<DedupPoolId<HirType>> {
        if received == expected {
            return Ok(received);
        }
        match (
            queue.hir.view(received).dereference(),
            queue.hir.view(expected).dereference(),
        ) {
            (a, b) if *a == *b => Ok(a.data),
            (a, b)
                if let Some(inner) = a.is_nullable()
                    && inner == b.data =>
            {
                Ok(a.data)
            }
            (b, a)
                if let Some(inner) = a.is_nullable()
                    && inner == b.data =>
            {
                Ok(a.data)
            }
            (a, b) if let HirType::GenericParam { .. } = a.raw() => Ok(b.data),
            (b, a) if let HirType::GenericParam { .. } = a.raw() => Ok(b.data),
            (received, expected) => Err(HIRError::unexpected_type(
                received.data,
                expected.data,
                span,
            )),
        }
    }

    fn is_expression_able_to_write(
        &self,
        queue: &HirQueueBuilder,
        expr: Spanned<PoolId<HirExpression>>,
    ) -> Result<()> {
        let expression = &queue.hir[expr.data];
        match expression.kind {
            HirExpressionKind::Identifier(ident) => {
                if self.is_mutable(ident) {
                    Ok(())
                } else {
                    let ident = self
                        .names
                        .iter()
                        .find_map(|entry| (*entry.1 == ident).then_some(*entry.0))
                        .expect(
                            "name of variable should be visible. Something is creating a variable on function builders, but for some reason not defining them on the builder names",
                        );
                    Err(HIRError::invalid_variable_write(ident, expr.span))
                }
            }

            HirExpressionKind::FieldAccess { expr, .. } => {
                self.is_expression_able_to_write(queue, expr)
            }
            _ => Err(HIRError::invalid_expr_write(expr.span)),
        }
    }

    fn lookup_function(
        &self,
        queue: &HirQueueBuilder<'_>,
        name: Spanned<SymbolPointer>,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        let identifier = name.data;

        if let Some(func) = queue
            .hir
            .find_function_by_symbol(HirSymbol::new(self.file(), identifier))
        {
            Ok(func)
        } else if let Some(func) = queue
            .hir
            .get_file(self.file())
            .find_function_with_name(identifier)
        {
            Ok(func)
        } else {
            Err(HIRError::name_unrecognized(identifier, name.span))
        }
    }

    fn build_field_access(
        &mut self,
        queue: &HirQueueBuilder,
        parent: Spanned<PoolId<HirExpression>>,
        field_ast: Spanned<DedupPoolId<ASTExpression>>,
        span: Span,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        let expr = match queue.get_expr(field_ast.data) {
            ASTExpression::FieldAccess {
                parent: inner_parent,
                field: inner_field,
            } => {
                let intermediate =
                    self.build_field_access(queue, parent, *inner_parent, span, context)?;
                return self.build_field_access(queue, intermediate, *inner_field, span, context);
            }
            ASTExpression::Identifier(field_name) => {
                let parent_ty = queue.hir[parent.data].ty;
                let parent_view = queue.hir.view(parent_ty);
                let resolved = parent_view.dereference();
                match resolved.is_struct() {
                    None => {
                        let ty = (*queue.hir.view(resolved.data)).clone();
                        return Err(HIRError::not_a_struct(ty, span));
                    }
                    Some(view) => {
                        let (fields, field_types) = (view.fields(), view.field_types());
                        let position = fields
                            .iter()
                            .position(|f| f.data == *field_name)
                            .ok_or_else(|| {
                                HIRError::property_unrecognized(
                                    resolved.data,
                                    vec![*field_name],
                                    span,
                                )
                            })?;

                        HirExpression {
                            ty: field_types[position],
                            kind: HirExpressionKind::FieldAccess {
                                expr: parent,
                                field_index: position,
                                field_name: Some(*field_name),
                            },
                        }
                    }
                }
            }
            ASTExpression::FunctionCall { name, args } => {
                let name_sym = queue.type_name(name.data, &TypeContext::EMPTY);
                let parent_ty = queue.hir[parent.data].ty;
                let real_ty = queue.hir.view(parent_ty);
                let deref = real_ty.dereference();
                match deref.is_struct() {
                    None => return Err(HIRError::not_a_struct(deref.raw().clone(), span)),
                    Some(view) => {
                        let func_id =
                            view.methods()
                                .iter()
                                .find_map(
                                    |Visible {
                                         data: (method, func),
                                         visibility,
                                     }| {
                                        (*method == name_sym
                                            && *visibility == VisibilityModifier::Public)
                                            .then_some(*func)
                                    },
                                )
                                .or_else(|| {
                                    queue.hir.methods.get(&deref.data).and_then(|methods| {
                                        methods.get(&name_sym).map(|v| *v.value())
                                    })
                                });

                        let func_id = match func_id {
                            Some(id) => id,
                            None if let Some(id) =
                                queue.resolve_method(self.file(), deref.data, name_sym)? =>
                            {
                                id
                            }
                            _ => {
                                return Err(HIRError::missing_properties(vec![name_sym], span));
                            }
                        };

                        let func_view = queue.hir.view(func_id);
                        let expected = func_view
                            .ty()
                            .is_function()
                            .expect("Method should be a function")
                            .arguments()
                            .to_vec();
                        if expected.len() != args.len() + 1 {
                            //+1 due to being a method call, so self is implicit
                            return Err(HIRError::invalid_funcall_arg_length(
                                name_sym,
                                expected.len(),
                                args.len(),
                                name.span,
                            ));
                        }
                        let built_args = args
                            .iter()
                            .enumerate()
                            .map(|(idx, arg)| {
                                self.build_expression(queue, *arg, Some(expected[idx]), context)
                            })
                            .collect::<Result<Vec<_>>>()?;
                        let mut method_args = vec![parent];
                        method_args.extend(built_args);
                        HirExpression {
                            ty: func_view.return_type(),
                            kind: HirExpressionKind::FunctionCall {
                                name: func_id,
                                args: method_args,
                                generics: Vec::new(),
                            },
                        }
                    }
                }
            }
            _ => return Err(HIRError::invalid_field_access(span)),
        };
        Ok(span.make_spanned(queue.hir.insert_expression(expr)))
    }

    fn build_identifier(
        &self,
        queue: &HirQueueBuilder,
        name: SymbolPointer,
        expression_span: Span,
    ) -> Result<HirExpression> {
        match self.resolve_name(queue, name, expression_span)? {
            HirName::Variable(v) => {
                let ty = *self
                    .variables_types
                    .get(&v)
                    .expect("Expected variable to have a type defined on this builder");
                Ok(HirExpression {
                    ty,
                    kind: HirExpressionKind::Identifier(v),
                })
            }
            HirName::Static(s) => {
                let ty = queue.hir.get_static(s).ty;
                Ok(HirExpression {
                    ty,
                    kind: HirExpressionKind::Static { id: s },
                })
            }
        }
    }
    fn build_tuple_expression(
        &mut self,
        queue: &HirQueueBuilder,
        fields: &[Spanned<DedupPoolId<ASTExpression>>],
        expected: Option<DedupPoolId<HirType>>,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let mut expressions = Vec::with_capacity(fields.len());
        let mut types = Vec::with_capacity(fields.len());

        for (idx, field) in fields.iter().enumerate() {
            let field_type = if let Some(expected) = expected
                && let Some(tuple) = queue.hir.view(expected).is_tuple()
            {
                Some(tuple.fields()[idx])
            } else {
                None
            };
            let expr = self.build_expression(queue, *field, field_type, context)?;
            types.push(queue.hir[expr.data].ty);
            expressions.push(expr);
        }
        Ok(HirExpression {
            ty: queue.hir.create_tuple_type(types),
            kind: HirExpressionKind::Tuple(expressions),
        })
    }

    fn build_tuple_access(
        &mut self,
        queue: &HirQueueBuilder,
        tuple: Spanned<DedupPoolId<ASTExpression>>,
        expected: Option<DedupPoolId<HirType>>,
        span: Span,
        index: usize,
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let expr = self.build_expression(queue, tuple, expected, context)?;
        let raw_expr = &queue.hir[expr.data];
        let parent_view = queue.hir.view(raw_expr.ty);
        let resolved = parent_view.dereference();
        let ty = match resolved.is_tuple() {
            None => {
                let ty = (*resolved).clone();
                return Err(HIRError::not_a_tuple(ty, span));
            }
            Some(tuple_view) => {
                let field_index = index;
                let fields = tuple_view.fields();
                if field_index >= fields.len() {
                    return Err(HIRError::invalid_tuple_index(
                        field_index,
                        fields.len(),
                        span,
                    ));
                }
                fields[field_index]
            }
        };
        Ok(HirExpression {
            ty,
            kind: HirExpressionKind::FieldAccess {
                expr,
                field_index: index,
                field_name: None,
            },
        })
    }

    pub(crate) fn build_expression(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        expression: Spanned<DedupPoolId<ASTExpression>>,
        expected: Option<DedupPoolId<HirType>>,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        let expr = queue.get_expr(expression.data);
        let expr = match expr {
            ASTExpression::Null => {
                let ty = match expected {
                    None => {
                        return Err(HIRError::couldnt_infer(expression.span));
                    }
                    Some(ty) if let HirType::Nullable(_) = queue.hir.deref()[ty] => ty,
                    Some(ty) => {
                        return Err(HIRError::unexpected_type(
                            ty,
                            queue.hir.create_type(HirType::Nullable(ty)),
                            expression.span,
                        ));
                    }
                };
                HirExpression {
                    ty,
                    kind: HirExpressionKind::Null,
                }
            }
            ASTExpression::IndexExpression(expr, range) => {
                let expr = self.build_expression(queue, *expr, expected, context)?;
                let after_index_type = {
                    let expr_type = queue.hir[expr.data].ty;
                    match &queue.hir.deref()[expr_type] {
                        HirType::Vector(t) => *t,
                        HirType::Array(t, _) => *t,
                        HirType::GenericParam { .. } => expr_type,
                        _ => return Err(HIRError::invalid_indexing(expr_type, expression.span)),
                    }
                };
                match range {
                    RangeType::NoRange(index) => {
                        let index = self.build_expression(queue, *index, expected, context)?;
                        let viewer = queue.hir.view(index.data);
                        let ty_viewer = viewer.ty_viewer();
                        match ty_viewer.raw() {
                            HirType::Int => {}
                            _ => {
                                return Err(HIRError::unexpected_type(
                                    ty_viewer.data,
                                    queue.hir.create_type(HirType::Int),
                                    index.span,
                                ));
                            }
                        }
                        HirExpression {
                            kind: HirExpressionKind::ArrayIndex(expr, index),
                            ty: after_index_type,
                        }
                    }
                    r => unimplemented!("Ranges {r:?} are not implemented yet"),
                }
            }
            ASTExpression::False => HirExpression {
                ty: queue.hir.create_type(HirType::Bool),
                kind: HirExpressionKind::False,
            },
            ASTExpression::True => HirExpression {
                ty: queue.hir.create_type(HirType::Bool),
                kind: HirExpressionKind::True,
            },
            ASTExpression::Identifier(name) => {
                self.build_identifier(queue, *name, expression.span)?
            }
            ASTExpression::IntLiteral(i) => queue.hir.create_int_expression(*i, 0),
            ASTExpression::FloatLiteral(f) => queue.hir.create_float_expression(f.into_inner()),
            ASTExpression::StringLiteral(s) => queue.hir.create_strliteral_expression(*s),
            ASTExpression::Tuple(fields) => {
                self.build_tuple_expression(queue, fields, expected, context)?
            }

            ASTExpression::FieldAccess { parent, field } => {
                let parent = self.build_expression(queue, *parent, expected, context)?;
                return self.build_field_access(queue, parent, *field, expression.span, context);
            }
            ASTExpression::TupleAccess { tuple, index } => self.build_tuple_access(
                queue,
                *tuple,
                expected,
                expression.span,
                *index as usize,
                context,
            )?,
            ASTExpression::Binary { lhs, op, rhs } => {
                let lhs = self.build_expression(queue, *lhs, expected, context)?;
                let rhs = self.build_expression(queue, *rhs, expected, context)?;
                let lhs_ty = queue.hir.view(lhs.data).ty();
                let rhs_ty = queue.hir.view(rhs.data).ty();
                let ty = self.unify_types(queue, lhs_ty, rhs_ty, expression.span)?;
                let ty = if op.is_logical() {
                    queue.hir.create_type(HirType::Bool)
                } else {
                    ty
                };
                queue.hir.create_binary_expression(lhs, rhs, *op, ty)
            }
            ASTExpression::FunctionCall { name, args } => {
                let identifier = queue.get_plain_type(*name);
                let func =
                    self.lookup_function(queue, name.span.make_spanned(identifier.identifier))?;
                let func_viewer = queue.hir.view(func);
                let func_ty_view = func_viewer.ty();
                let func_real_type = func_ty_view
                    .is_function()
                    .expect("Function should have function type");

                let expected_args = func_real_type.arguments();

                if expected_args.len() != args.len() {
                    let func_name = identifier.identifier;
                    return Err(HIRError::invalid_funcall_arg_length(
                        func_name,
                        expected_args.len(),
                        args.len(),
                        name.span,
                    ));
                }
                let generics = queue
                    .get_node(self.file())
                    .resolve_call_generics(identifier, context)?;
                let args = args
                    .iter()
                    .zip(expected_args)
                    .map(|(arg, ty)| {
                        let ty = match queue.hir.view(*ty).raw() {
                            HirType::GenericParam { index, .. } => {
                                generics.get(*index as usize).copied().unwrap_or(*ty)
                            }
                            _ => *ty,
                        };

                        self.build_expression(queue, *arg, Some(ty), context)
                    })
                    .collect::<Result<_>>()?;
                let ty = func_real_type.return_type();
                HirExpression {
                    kind: HirExpressionKind::FunctionCall {
                        name: func,
                        args,
                        generics,
                    },
                    ty,
                }
            }
            ASTExpression::If {
                condition,
                body,
                else_body,
            } => {
                let condition = self.build_expression(queue, *condition, expected, context)?;
                let bool_ty = queue.hir.create_type(HirType::Bool);
                self.unify_types(
                    queue,
                    queue.hir[condition.data].ty,
                    bool_ty,
                    expression.span,
                )?;

                let then_branch = body
                    .iter()
                    .map(|stmt| self.build_statement(queue, stmt, context))
                    .collect::<Result<Vec<_>>>()?;
                let else_branch = if else_body.is_empty() {
                    None
                } else {
                    Some(
                        else_body
                            .iter()
                            .map(|stmt| self.build_statement(queue, stmt, context))
                            .collect::<Result<Vec<_>>>()?,
                    )
                };

                let then_ty = then_branch
                    .last()
                    .map(|s| match &queue.hir[s.data] {
                        HirStatement::Expression { expr } => queue.hir[expr.data].ty,
                        HirStatement::Variable { value, .. } => queue.hir[value.data].ty,
                        _ => queue.hir.create_type(HirType::Void),
                    })
                    .unwrap_or_else(|| queue.hir.create_type(HirType::Void));
                let else_ty = else_branch
                    .as_ref()
                    .and_then(|b| b.last())
                    .map(|s| match &queue.hir[s.data] {
                        HirStatement::Expression { expr } => queue.hir[expr.data].ty,
                        HirStatement::Variable { value, .. } => queue.hir[value.data].ty,
                        _ => queue.hir.create_type(HirType::Void),
                    })
                    .unwrap_or_else(|| queue.hir.create_type(HirType::Void));
                self.unify_types(queue, else_ty, then_ty, expression.span)?;
                HirExpression {
                    ty: then_ty,
                    kind: HirExpressionKind::If {
                        condition,
                        then_branch,
                        else_branch,
                    },
                }
            }
            ASTExpression::Component(component) => {
                let child =
                    self.build_component_expression(queue, component, expression.span, context)?;
                HirExpression {
                    ty: queue.hir[child.data].name,
                    kind: HirExpressionKind::Component(child),
                }
            }
            ASTExpression::ObjectExpression { name, fields } => {
                let (_, ty) = queue.get_node(self.file()).find_type(*name, context)?;
                let ty_view = queue.hir.view(ty);
                let deref = ty_view.dereference();
                let obj = deref
                    .is_struct()
                    .expect("Expected name to generate a struct type");

                let type_names: HashMap<_, _> = obj
                    .fields()
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (s.data, (i, s.visibility)))
                    .collect();
                let mut ordered = vec![None; obj.fields().len()];

                for field in fields {
                    let (idx, visibility) = *type_names.get(&field.data.name).ok_or(
                        HIRError::property_unrecognized(ty, vec![field.data.name], field.span),
                    )?;
                    if visibility != VisibilityModifier::Public {
                        return Err(HIRError::not_visible_property(field.data.name, field.span));
                    }

                    if ordered[idx].replace(field).is_some() {
                        return Err(HIRError::already_defined(field.data.name, field.span));
                    }
                }

                let fields = {
                    let mut fields = Vec::with_capacity(ordered.len());

                    let mut missing = Vec::new();
                    for (i, field) in ordered.into_iter().enumerate() {
                        match field {
                            Some(field) => fields.push({
                                let fieldname = field.data.name;
                                let (idx, _) = type_names
                                    .get(&fieldname)
                                    .expect("Field name should've been added into type names");

                                self.build_expression(
                                    queue,
                                    field.data.expr,
                                    Some(obj.field_types()[*idx]),
                                    context,
                                )?
                            }),
                            None => missing.push(obj.fields()[i].data),
                        }
                    }

                    if !missing.is_empty() {
                        return Err(HIRError::missing_properties(missing, expression.span));
                    }
                    fields
                };

                HirExpression {
                    ty,
                    kind: HirExpressionKind::Object { name: ty, fields },
                }
            }
            ASTExpression::Array(expressions) => {
                let mut exprs = Vec::with_capacity(expressions.len());
                let Some(first) = expressions.first() else {
                    return match expected {
                        Some(ty) if queue.hir.view(ty).is_array().is_some() => {
                            let expr = queue.hir.insert_expression(HirExpression {
                                ty,
                                kind: HirExpressionKind::Array(exprs),
                            });
                            Ok(expression.span.make_spanned(expr))
                        }
                        Some(_) | None => Err(HIRError::couldnt_infer(expression.span)),
                    };
                };
                let (inner_type, size) =
                    expected.and_then(|e| queue.hir.view(e).is_array()).unzip();
                let expr = self.build_expression(queue, *first, inner_type, context)?;
                let ty = queue.hir[expr.data].ty;
                if let Some(expected) = inner_type {
                    self.unify_types(queue, ty, expected, expression.span)?;
                }
                exprs.push(expr);
                for expr in &expressions[1..] {
                    let expr = self.build_expression(queue, *expr, Some(ty), context)?;
                    exprs.push(expr);
                }
                let final_length = size.unwrap_or(exprs.len());
                if let Some(expected_len) = size
                    && final_length != expected_len
                {
                    return Err(HIRError::array_length_mismatch(
                        expected_len,
                        final_length,
                        expression.span,
                    ));
                }
                let final_type = queue.hir.create_type(HirType::Array(ty, final_length));
                HirExpression {
                    ty: final_type,
                    kind: HirExpressionKind::Array(exprs),
                }
            }
            ASTExpression::Vector(expressions) => {
                let mut exprs = Vec::with_capacity(expressions.len());
                let Some(first) = expressions.first() else {
                    return match expected {
                        Some(ty) if queue.hir.view(ty).is_vector().is_some() => {
                            let expr = queue.hir.insert_expression(HirExpression {
                                ty,
                                kind: HirExpressionKind::Vector(exprs),
                            });
                            Ok(expression.span.make_spanned(expr))
                        }
                        Some(_) | None => Err(HIRError::couldnt_infer(expression.span)),
                    };
                };
                let inner_type = expected.and_then(|e| queue.hir.view(e).is_vector());
                let expr = self.build_expression(queue, *first, inner_type, context)?;
                let ty = queue.hir[expr.data].ty;
                if let Some(expected) = inner_type {
                    self.unify_types(queue, ty, expected, expression.span)?;
                }
                exprs.push(expr);
                for expr in &expressions[1..] {
                    let expr = self.build_expression(queue, *expr, Some(ty), context)?;
                    exprs.push(expr);
                }
                let final_type = queue.hir.create_type(HirType::Vector(ty));
                HirExpression {
                    ty: final_type,
                    kind: HirExpressionKind::Vector(exprs),
                }
            }
        };
        Ok(expression
            .span
            .make_spanned(queue.hir.insert_expression(expr)))
    }

    pub fn build_statement(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        statement: &Spanned<DedupPoolId<ASTStatement>>,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirStatement>>> {
        let (data, span) = self.build_statement_data(queue, statement, context)?;
        let id = queue.hir.insert_statement(data);
        Ok(span.make_spanned(id))
    }

    /// Builds a statement and returns the raw `HirStatement` without inserting
    /// into the pool. Used for the last statement in function bodies where we
    /// may need to wrap it in an implicit return.
    pub(crate) fn build_statement_data(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        statement: &Spanned<DedupPoolId<ASTStatement>>,
        context: &TypeContext,
    ) -> Result<(HirStatement, Span)> {
        let stmt = queue.get_statement(statement.data);
        let data = match stmt {
            ASTStatement::Expression(e) => {
                let expr = self.build_expression(queue, *e, None, context)?;
                HirStatement::Expression { expr }
            }
            ASTStatement::Var { name, ty, rhs } | ASTStatement::MutableVar { name, ty, rhs } => {
                let var_type = if let Some(ty) = ty {
                    Some(queue.get_node(self.file()).find_type(*ty, context)?.1)
                } else {
                    None
                };
                let expr = self.build_expression(queue, *rhs, var_type, context)?;
                let exprty = queue.hir.view(expr.data).ty();
                let expected_type = if let Some(expected_ty) = ty {
                    queue
                        .get_node(self.file())
                        .find_type(*expected_ty, context)?
                        .1
                } else {
                    exprty
                };
                let ty = self.unify_types(queue, exprty, expected_type, statement.span)?;
                let varid = self.create_variable(
                    *name,
                    matches!(stmt, ASTStatement::MutableVar { .. }),
                    exprty,
                );

                self.variables_types.insert(varid, ty);
                HirStatement::Variable {
                    name: varid,
                    value: expr,
                }
            }
            ASTStatement::Assign { lhs, rhs } => {
                let lhs = self.build_expression(queue, *lhs, None, context)?;
                self.is_expression_able_to_write(queue, lhs)?;
                let rhs = self.build_expression(queue, *rhs, None, context)?;
                self.unify_types(
                    queue,
                    queue.hir.view(rhs.data).ty(),
                    queue.hir.view(lhs.data).ty(),
                    statement.span,
                )?;
                HirStatement::Assign { lhs, value: rhs }
            }
            ASTStatement::While { condition, body } => {
                let condition = self.build_expression(
                    queue,
                    *condition,
                    Some(queue.hir.create_type(HirType::Bool)),
                    context,
                )?;
                let body = body
                    .iter()
                    .map(|statement| self.build_statement(queue, statement, context))
                    .collect::<Result<_>>()?;

                HirStatement::While { condition, body }
            }
            ASTStatement::Return { value } => {
                let expr = value
                    .map(|v| self.build_expression(queue, v, None, context))
                    .transpose()?;
                HirStatement::Return { expr }
            }
        };
        Ok((data, statement.span))
    }

    pub fn build_component_expression(
        &mut self,
        queue: &HirQueueBuilder,
        component: &ComponentExpression,
        span: Span,
        context: &TypeContext,
    ) -> Result<Spanned<PoolId<HirComponentExpression>>> {
        let name = queue.get_plain_type(component.name).identifier;
        let node = queue.get_node(self.file());
        let (owner, ty) = node.find_type(component.name, context)?;
        if queue
            .hir
            .find_component_by_symbol(HirSymbol::new(owner, name))
            .is_none()
        {
            let ty = queue.modules.find_type_inside_module(self.file(), name);
            if let Some(ASTType {
                content: ASTTypeKind::Component(comp),
                ..
            }) = ty
            {
                queue.enqueue_component(comp, self.file())?;
            } else {
                return Err(HIRError::component_not_found(name, span));
            }
        }
        let ty_view = queue.hir.view(ty);
        let deref = ty_view.dereference();
        let comp_view = deref
            .is_component()
            .expect("find_type returned non-component type for component name");

        let mut properties = Vec::new();
        let mut children = Vec::new();
        for value in &component.values {
            match value {
                ComponentMemberValue::Assign { prop_name, rhs } => {
                    let pos = comp_view
                        .prop_names()
                        .iter()
                        .position(|n| n == prop_name)
                        .ok_or_else(|| {
                            HIRError::property_unrecognized(ty, vec![*prop_name], span)
                        })?;
                    let expr =
                        self.build_expression(queue, *rhs, Some(comp_view.props()[pos]), context)?;
                    properties.push(PropertyExpression::new(pos, expr));
                }
                ComponentMemberValue::Child(child) => {
                    let child_expr =
                        self.build_component_expression(queue, child, span, context)?;
                    children.push(child_expr);
                }
            }
        }

        let component_expr = HirComponentExpression {
            name: ty,
            properties,
            children,
        };
        let id = queue.hir.insert_component_expression(component_expr);
        Ok(span.make_spanned(id))
    }
}
