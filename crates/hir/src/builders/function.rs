use std::collections::{HashMap, HashSet};

use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use slynx_parser::{
    ASTExpression, ASTStatement, ComponentExpression, ComponentMemberValue, GenericIdentifier,
};

use crate::{
    DeclarationId, HIRError, HirComponentExpression, HirExpression, HirExpressionKind,
    HirFunctionDeclaration, HirStatement, HirStaticDeclaration, HirType, PropertyExpression,
    Result, SymbolPointer, VariableId, builders::HirQueueBuilder, context::HirSymbol,
    error::InvalidTypeReason,
};

pub struct HirFunctionBuildResult {
    pub(crate) args: Vec<VariableId>,
    pub(crate) statements: Vec<Spanned<PoolId<HirStatement>>>,
}

pub struct HirFunctionBuilder {
    pub(crate) target: DeclarationId<HirFunctionDeclaration>,
    pub(crate) names: HashMap<SymbolPointer, VariableId>,
    pub(crate) variables_types: HashMap<VariableId, DedupPoolId<HirType>>,
    pub(crate) mutable: HashSet<VariableId>,
}

pub enum HirName {
    Variable(VariableId),
    Static(DeclarationId<HirStaticDeclaration>),
}

impl HirFunctionBuilder {
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
                let name = queue.hir.intern_name(&format!("{:?}", &*received));
                Err(HIRError::invalid_type(
                    name,
                    InvalidTypeReason::IncorrectUsage,
                    span,
                ))
            }
        }
    }

    fn is_mutable(&self, id: VariableId) -> bool {
        self.mutable.contains(&id)
    }

    fn create_variable(&mut self, name: SymbolPointer, mutable: bool) -> VariableId {
        let id = VariableId::new(self.target, self.names.len() as u8);
        self.names.insert(name, id);
        if mutable {
            self.mutable.insert(id);
        }
        id
    }

    pub(crate) fn create_argument(
        &mut self,
        queue: &HirQueueBuilder,
        name: SymbolPointer,
        arg_index: u8,
    ) {
        let (id, ty) = queue
            .hir
            .view(self.target)
            .get_argument(arg_index)
            .expect("Argument index should be < function argument count");
        self.names.insert(name, id);
        self.variables_types.insert(id, ty);
    }

    pub(crate) fn get_variable<'a>(
        &self,
        queue: &HirQueueBuilder,
        ptr: SymbolPointer,
        span: Span,
    ) -> Result<HirName> {
        if let Some(var) = self.names.get(&ptr).cloned() {
            Ok(HirName::Variable(var))
        } else if let Some((file_owner, statik)) =
            queue.find_static_declaration(ptr, queue.get_entry(self.target.file_id))
        {
            let id = queue.enqueue_static(statik, queue.get_node(file_owner))?;
            Ok(HirName::Static(id))
        } else {
            Err(HIRError::name_unrecognized(ptr, span))
        }
    }
}

impl HirFunctionBuilder {
    pub fn new(target: DeclarationId<HirFunctionDeclaration>) -> Self {
        Self {
            target,
            names: HashMap::new(),
            variables_types: HashMap::new(),
            mutable: HashSet::new(),
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
                        .find_map(|entry| (*entry.1 == ident).then_some(*entry.0)).expect("name of variable should be visible. Something is creating a variable on function builders, but for some reason not defining them on the builder names");
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
        name: Spanned<DedupPoolId<GenericIdentifier>>,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        let identifier = queue.get_type(name.data).identifier;
        let file_id = self.target.file_id;
        if let Some(func) = queue
            .hir
            .find_function_by_symbol(HirSymbol::new(file_id, identifier))
        {
            Ok(func)
        } else if let Some(func) = queue
            .hir
            .get_file(file_id)
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
                if let Some(view) = resolved.is_struct() {
                    let (fields, field_types) = (view.fields(), view.field_types());
                    let position =
                        fields
                            .iter()
                            .position(|f| *f == *field_name)
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
                } else {
                    let ty = (*queue.hir.view(resolved.data)).clone();
                    return Err(HIRError::not_a_struct(ty, span));
                }
            }
            ASTExpression::FunctionCall { name, args } => {
                let name_sym = queue.get_type(name.data).identifier;
                let parent_ty = queue.hir[parent.data].ty;
                let real_ty = queue.hir.view(parent_ty);
                let deref = real_ty.dereference();
                if let Some(view) = deref.is_struct() {
                    let func_id = view
                        .methods()
                        .iter()
                        .find_map(|(method, func)| (*method == name_sym).then_some(*func))
                        .or_else(|| {
                            queue
                                .hir
                                .methods
                                .get(&deref.data)
                                .and_then(|methods| methods.get(&name_sym).map(|v| *v.value()))
                        });

                    let func_id = match func_id {
                        Some(id) => id,
                        None => {
                            match queue.resolve_method(
                                self.target.file_id,
                                deref.data,
                                name_sym,
                                span,
                            )? {
                                Some(id) => id,
                                None => {
                                    return Err(HIRError::missing_properties(
                                        vec![name_sym],
                                        span,
                                    ))
                                }
                            }
                        }
                    };

                    let func_view = queue.hir.view(func_id);
                    let expected = func_view
                        .ty()
                        .is_function()
                        .expect("Method should be a function")
                        .arguments()
                        .len();
                    let received = args.len() + 1;
                    if expected != received {
                        return Err(HIRError::invalid_funcall_arg_length(
                            name_sym,
                            expected,
                            received,
                            name.span,
                        ));
                    }
                    let built_args = args
                        .iter()
                        .map(|a| self.build_expression(queue, *a))
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
                } else {
                    return Err(HIRError::not_a_struct(
                        deref.raw().clone(),
                        span,
                    ));
                }
            }
            _ => return Err(HIRError::invalid_field_access(span)),
        };
        Ok(span.make_spanned(queue.hir.insert_expression(expr)))
    }

    pub(crate) fn build_expression(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        expression: Spanned<DedupPoolId<ASTExpression>>,
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
                match self.get_variable(queue, *name, expression.span)? {
                    HirName::Variable(v) => {
                        let ty = self
                            .variables_types
                            .get(&v)
                            .expect("Expected variable to have a type defined on this builder")
                            .clone();
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
            ASTExpression::IntLiteral(i) => queue.hir.create_int_expression(*i, 0),
            ASTExpression::FloatLiteral(f) => queue.hir.create_float_expression(f.into_inner()),
            ASTExpression::StringLiteral(s) => queue.hir.create_strliteral_expression(*s),
            ASTExpression::Tuple(fields) => {
                let mut expressions = Vec::with_capacity(fields.len());
                let mut types = Vec::with_capacity(fields.len());
                for field in fields {
                    let expr = self.build_expression(queue, *field)?;
                    types.push(queue.hir[expr.data].ty);
                    expressions.push(expr);
                }
                HirExpression {
                    ty: queue.hir.create_tuple_type(types),
                    kind: HirExpressionKind::Tuple(expressions),
                }
            }
            ASTExpression::FieldAccess { parent, field } => {
                let parent = self.build_expression(queue, *parent)?;
                return self.build_field_access(queue, parent, *field, expression.span);
            }
            ASTExpression::TupleAccess { tuple, index } => {
                let expr = self.build_expression(queue, *tuple)?;
                let raw_expr = &queue.hir[expr.data];
                let parent_view = queue.hir.view(raw_expr.ty);
                let resolved = parent_view.dereference();
                let ty = if let Some(tuple_view) = resolved.is_tuple() {
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
                } else {
                    let ty = (*resolved).clone();
                    return Err(HIRError::not_a_tuple(ty, expression.span));
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
                let lhs = self.build_expression(queue, *lhs)?;
                let rhs = self.build_expression(queue, *rhs)?;
                let lhs_ty = queue.hir.view(lhs.data).ty();
                let rhs_ty = queue.hir.view(rhs.data).ty();
                let ty = self.unify_types(queue, lhs_ty, rhs_ty, expression.span)?;
                queue.hir.create_binary_expression(lhs, rhs, *op, ty)
            }
            ASTExpression::FunctionCall { name, args } => {
                let func = self.lookup_function(queue, *name)?;
                let func_view = queue.hir.view(func);
                let expected = func_view
                    .ty()
                    .is_function()
                    .expect("Function should have function type")
                    .arguments()
                    .len();
                let received = args.len();
                if expected != received {
                    let func_name = queue.get_type(name.data).identifier;
                    return Err(HIRError::invalid_funcall_arg_length(
                        func_name,
                        expected,
                        received,
                        name.span,
                    ));
                }
                let args = args
                    .iter()
                    .map(|f| self.build_expression(queue, *f))
                    .collect::<Result<_>>()?;
                let ty = func_view.return_type();
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
                let condition = self.build_expression(queue, *condition)?;
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
                let child = self.build_component_expression(queue, &component, expression.span)?;
                HirExpression {
                    ty: queue.hir[child.data].name,
                    kind: HirExpressionKind::Component(child),
                }
            }
            ASTExpression::ObjectExpression { name, fields } => {
                let (_, ty) = queue.get_node(self.target.file_id).find_type(*name)?;
                let ty_view = queue.hir.view(ty);
                let obj = ty_view
                    .is_struct()
                    .expect("Expected name to generate a struct type");

                let type_names: HashMap<_, _> = obj
                    .fields()
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (s, i))
                    .collect();
                let mut ordered = vec![None; obj.fields().len()];

                for field in fields {
                    let idx = *type_names.get(&field.data.name).ok_or(
                        HIRError::property_unrecognized(ty, vec![field.data.name], field.span),
                    )?;

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
                                let expr = self.build_expression(queue, field.data.expr)?;
                                expr
                            }),
                            None => missing.push(obj.fields()[i]),
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

    fn build_statement(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        statement: &Spanned<DedupPoolId<ASTStatement>>,
    ) -> Result<Spanned<PoolId<HirStatement>>> {
        let stmt = queue.get_statement(statement.data);
        let id = match stmt {
            ASTStatement::Expression(e) => {
                let expr = self.build_expression(queue, *e)?;
                queue
                    .hir
                    .insert_statement(HirStatement::Expression { expr })
            }
            ASTStatement::Var { name, ty, rhs } | ASTStatement::MutableVar { name, ty, rhs } => {
                let varid =
                    self.create_variable(*name, matches!(stmt, ASTStatement::MutableVar { .. }));
                let expr = self.build_expression(queue, *rhs)?;

                let exprty = queue.hir.view(expr.data).ty();
                let expected_type = if let Some(expected_ty) = ty {
                    queue
                        .get_node(self.target.file_id)
                        .find_type(*expected_ty)?
                        .1
                } else {
                    exprty
                };
                let ty = self.unify_types(queue, exprty, expected_type, statement.span)?;
                self.variables_types.insert(varid, ty);
                queue.hir.insert_statement(HirStatement::Variable {
                    name: varid,
                    value: expr,
                })
            }
            ASTStatement::Assign { lhs, rhs } => {
                let lhs = self.build_expression(queue, *lhs)?;
                self.is_expression_able_to_write(queue, lhs)?;
                let rhs = self.build_expression(queue, *rhs)?;
                self.unify_types(
                    queue,
                    queue.hir.view(rhs.data).ty(),
                    queue.hir.view(lhs.data).ty(),
                    statement.span,
                )?;
                queue
                    .hir
                    .insert_statement(HirStatement::Assign { lhs, value: rhs })
            }
            ASTStatement::While { condition, body } => {
                let condition = self.build_expression(queue, *condition)?;
                let body = body
                    .iter()
                    .map(|statement| self.build_statement(queue, statement))
                    .collect::<Result<_>>()?;

                queue
                    .hir
                    .insert_statement(HirStatement::While { condition, body })
            }
        };
        Ok(statement.span.make_spanned(id))
    }

    fn build_component_expression(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        component: &ComponentExpression,
        span: Span,
    ) -> Result<Spanned<PoolId<HirComponentExpression>>> {
        let node = queue.get_node(self.target.file_id);
        let (_, ty) = node.find_type(Spanned::new(component.name, span))?;
        let ty_view = queue.hir.view(ty);
        let comp_view = ty_view.dereference();
        let comp_id = match &*comp_view {
            HirType::Component(id) => *id,
            _ => unreachable!("find_type returned non-component type for component name"),
        };
        let def = queue.hir.get_component_definition(comp_id);

        let mut properties = Vec::new();
        let mut children = Vec::new();
        for value in &component.values {
            match value {
                ComponentMemberValue::Assign { prop_name, rhs } => {
                    let pos = def
                        .properties
                        .iter()
                        .position(|n| n == prop_name)
                        .ok_or_else(|| {
                            HIRError::property_unrecognized(ty, vec![*prop_name], span)
                        })?;
                    let expr = self.build_expression(queue, Spanned::new(*rhs, span))?;
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

    pub(crate) fn build_body(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        body: &[Spanned<DedupPoolId<ASTStatement>>],
    ) -> Result<HirFunctionBuildResult> {
        let args = self.names.iter().map(|v| *v.1).collect();
        let mut statements = Vec::new();
        for statement in body {
            let stmt = self.build_statement(queue, statement)?;
            statements.push(stmt);
        }

        Ok(HirFunctionBuildResult { args, statements })
    }
}
