use std::collections::{HashMap, HashSet};

use common::{
    Span, Spanned, VisibilityModifier,
    pool::{DedupPoolId, PoolId},
};
use module_loader::{ASTType, ASTTypeKind, FileId};
use slynx_parser::{
    ASTExpression, ASTStatement, ComponentExpression, ComponentMemberValue, Type,
};

use crate::{
    DeclarationId, HIRError, HirComponentExpression, HirExpression, HirExpressionKind,
    HirFunctionDeclaration, HirStatement, HirStaticDeclaration, HirType, PropertyExpression,
    Result, SymbolPointer, VariableId, builders::HirQueueBuilder, context::HirSymbol,
    error::InvalidTypeReason, helpers::Visible, id::OwnerId,
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
            (received, _) => {
                let name = queue.hir.intern_name(&format!("{:?}", *received));
                Err(HIRError::invalid_type(
                    name,
                    InvalidTypeReason::IncorrectUsage,
                    span,
                ))
            }
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
        name: Spanned<DedupPoolId<Type>>,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        let identifier = queue.type_name(name.data);

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
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        let expr = match queue.get_expr(field_ast.data) {
            ASTExpression::FieldAccess {
                parent: inner_parent,
                field: inner_field,
            } => {
                let intermediate = self.build_field_access(queue, parent, *inner_parent, span)?;
                return self.build_field_access(queue, intermediate, *inner_field, span);
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
                let name_sym = queue.type_name(name.data);
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
                                self.build_expression(queue, *arg, Some(expected[idx]))
                            })
                            .collect::<Result<Vec<_>>>()?;
                        let mut method_args = vec![parent];
                        method_args.extend(built_args);
                        HirExpression {
                            ty: func_view.return_type(),
                            kind: HirExpressionKind::FunctionCall {
                                name: func_id,
                                args: method_args,
                            },
                        }
                    }
                }
            }
            _ => return Err(HIRError::invalid_field_access(span)),
        };
        Ok(span.make_spanned(queue.hir.insert_expression(expr)))
    }

    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn build_expression(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        expression: Spanned<DedupPoolId<ASTExpression>>,
        expected: Option<DedupPoolId<HirType>>,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        let expr = queue.get_expr(expression.data);
        let expr = match expr {
            ASTExpression::False => HirExpression {
                ty: queue.hir.create_type(HirType::Bool),
                kind: HirExpressionKind::False,
            },
            ASTExpression::True => HirExpression {
                ty: queue.hir.create_type(HirType::Bool),
                kind: HirExpressionKind::True,
            },
            ASTExpression::Identifier(name) => {
                match self.resolve_name(queue, *name, expression.span)? {
                    HirName::Variable(v) => {
                        let ty = *self
                            .variables_types
                            .get(&v)
                            .expect("Expected variable to have a type defined on this builder");
                        HirExpression {
                            ty,
                            kind: HirExpressionKind::Identifier(v),
                        }
                    }
                    HirName::Static(s) => {
                        let ty = queue.hir.get_static(s).ty;
                        HirExpression {
                            ty,
                            kind: HirExpressionKind::Static { id: s },
                        }
                    }
                }
            }
            ASTExpression::IntLiteral(i) => queue.hir.create_int_expression(*i, 0), /*if let Some(expected) = expected {
            queue.hir.create_int_expression(*i, 0)
            }else {}*/
            ASTExpression::FloatLiteral(f) => queue.hir.create_float_expression(f.into_inner()),
            ASTExpression::StringLiteral(s) => queue.hir.create_strliteral_expression(*s),
            ASTExpression::Tuple(fields) => {
                let mut expressions = Vec::with_capacity(fields.len());
                let mut types = Vec::with_capacity(fields.len());
                for field in fields {
                    let expr = self.build_expression(queue, *field, expected)?;
                    types.push(queue.hir[expr.data].ty);
                    expressions.push(expr);
                }
                HirExpression {
                    ty: queue.hir.create_tuple_type(types),
                    kind: HirExpressionKind::Tuple(expressions),
                }
            }
            ASTExpression::FieldAccess { parent, field } => {
                let parent = self.build_expression(queue, *parent, expected)?;
                return self.build_field_access(queue, parent, *field, expression.span);
            }
            ASTExpression::TupleAccess { tuple, index } => {
                let expr = self.build_expression(queue, *tuple, expected)?;
                let raw_expr = &queue.hir[expr.data];
                let parent_view = queue.hir.view(raw_expr.ty);
                let resolved = parent_view.dereference();
                let ty = match resolved.is_tuple() {
                    None => {
                        let ty = (*resolved).clone();
                        return Err(HIRError::not_a_tuple(ty, expression.span));
                    }
                    Some(tuple_view) => {
                        let field_index = *index as usize;
                        let fields = tuple_view.fields();
                        if field_index >= fields.len() {
                            return Err(HIRError::invalid_tuple_index(
                                field_index,
                                fields.len(),
                                expression.span,
                            ));
                        }
                        fields[field_index]
                    }
                };
                HirExpression {
                    ty,
                    kind: HirExpressionKind::FieldAccess {
                        expr,
                        field_index: *index as usize,
                        field_name: None,
                    },
                }
            }
            ASTExpression::Binary { lhs, op, rhs } => {
                let lhs = self.build_expression(queue, *lhs, expected)?;
                let rhs = self.build_expression(queue, *rhs, expected)?;
                let lhs_ty = queue.hir.view(lhs.data).ty();
                let rhs_ty = queue.hir.view(rhs.data).ty();
                let ty = self.unify_types(queue, lhs_ty, rhs_ty, expression.span)?;
                queue.hir.create_binary_expression(lhs, rhs, *op, ty)
            }
            ASTExpression::FunctionCall { name, args } => {
                let func = self.lookup_function(queue, *name)?;
                let func_viewer = queue.hir.view(func);
                let func_ty_view = func_viewer.ty();
                let func_real_type = func_ty_view
                    .is_function()
                    .expect("Function should have function type");

                let expected_args = func_real_type.arguments();

                if expected_args.len() != args.len() {
                    let func_name = queue.type_name(name.data);
                    return Err(HIRError::invalid_funcall_arg_length(
                        func_name,
                        expected_args.len(),
                        args.len(),
                        name.span,
                    ));
                }
                let args = args
                    .iter()
                    .enumerate()
                    .map(|(idx, arg)| self.build_expression(queue, *arg, Some(expected_args[idx])))
                    .collect::<Result<_>>()?;
                let ty = func_real_type.return_type();
                HirExpression {
                    kind: HirExpressionKind::FunctionCall { name: func, args },
                    ty,
                }
            }
            ASTExpression::If {
                condition,
                body,
                else_body,
            } => {
                let condition = self.build_expression(queue, *condition, expected)?;
                let bool_ty = queue.hir.create_type(HirType::Bool);
                self.unify_types(
                    queue,
                    queue.hir[condition.data].ty,
                    bool_ty,
                    expression.span,
                )?;

                let then_branch = body
                    .iter()
                    .map(|stmt| self.build_statement(queue, stmt))
                    .collect::<Result<Vec<_>>>()?;
                let else_branch = if else_body.is_empty() {
                    None
                } else {
                    Some(
                        else_body
                            .iter()
                            .map(|stmt| self.build_statement(queue, stmt))
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
                let child = self.build_component_expression(queue, component, expression.span)?;
                HirExpression {
                    ty: queue.hir[child.data].name,
                    kind: HirExpressionKind::Component(child),
                }
            }
            ASTExpression::ObjectExpression { name, fields } => {
                let (_, ty) = queue.get_node(self.file()).find_type(*name)?;
                let ty_view = queue.hir.view(ty);
                let obj = ty_view
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
        };
        Ok(expression
            .span
            .make_spanned(queue.hir.insert_expression(expr)))
    }

    pub fn build_statement(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        statement: &Spanned<DedupPoolId<ASTStatement>>,
    ) -> Result<Spanned<PoolId<HirStatement>>> {
        let (data, span) = self.build_statement_data(queue, statement)?;
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
    ) -> Result<(HirStatement, Span)> {
        let stmt = queue.get_statement(statement.data);
        let data = match stmt {
            ASTStatement::Expression(e) => {
                let expr = self.build_expression(queue, *e, None)?;
                HirStatement::Expression { expr }
            }
            ASTStatement::Var { name, ty, rhs } | ASTStatement::MutableVar { name, ty, rhs } => {
                let expr = self.build_expression(queue, *rhs, None)?;
                let exprty = queue.hir.view(expr.data).ty();
                let expected_type = if let Some(expected_ty) = ty {
                    queue.get_node(self.file()).find_type(*expected_ty)?.1
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
                let lhs = self.build_expression(queue, *lhs, None)?;
                self.is_expression_able_to_write(queue, lhs)?;
                let rhs = self.build_expression(queue, *rhs, None)?;
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
                )?;
                let body = body
                    .iter()
                    .map(|statement| self.build_statement(queue, statement))
                    .collect::<Result<_>>()?;

                HirStatement::While { condition, body }
            }
            ASTStatement::Return { value } => {
                let expr = value
                    .map(|v| self.build_expression(queue, v, None))
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
    ) -> Result<Spanned<PoolId<HirComponentExpression>>> {
        let name = queue.type_name(component.name.data);
        let node = queue.get_node(self.file());
        let (owner, ty) = node.find_type(component.name)?;
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
                    let expr = self.build_expression(queue, *rhs, Some(comp_view.props()[pos]))?;
                    properties.push(PropertyExpression::new(pos, expr));
                }
                ComponentMemberValue::Child(child) => {
                    let child_expr = self.build_component_expression(queue, child, span)?;
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
