use std::collections::HashMap;

use common::{Span, Spanned, pool::PoolId};
use petgraph::visit::EdgeRef;
use slynx_hir::{
    ComponentMemberDeclaration, ComponentPropertyKind, DeclarationId, FileId, HirComponentDeclaration,
    HirComponentExpression, HirExpression, HirExpressionKind, SymbolPointer, VariableId,
    reactive_graph::DagNode, term::TermId,
};
use slynx_ir::{ComponentBuilder, ComponentValueBuilder, IRPointer, IRType, IRTypeId, SlynxIR, Value};

use crate::{
    CodegenError, CodegenErrorKind,
    lowerers::{LoweringState, functions::FunctionContext},
};

impl<'a> LoweringState<'a> {
    pub(crate) fn get_component_expression(
        &mut self,
        value: Spanned<PoolId<HirComponentExpression>>,
        ctx: &mut FunctionContext,
        member_values: Option<&HashMap<VariableId, Value>>,
    ) -> Result<Value, CodegenError> {
        let HirComponentExpression {
            name,
            properties,
            children,
            ..
        } = &self.hir[value.data];
        let (ty, all_values) = {
            let ty = self.types.get_mapped_type(name).ok_or(CodegenError::new(
                CodegenErrorKind::IRTypeNotRecognized(*name),
                value.span,
                ctx.file(),
            ))?;

            let mut all_values = Vec::new();
            let mut current_member_values: HashMap<VariableId, Value> = HashMap::new();
            if let Some(viewer) = self.hir.view(*name).is_component() {
                let props = viewer.props();
                let num_props = props.len();
                let mut prop_values = vec![Value::VOID; num_props];
                for prop in properties {
                    let val = if let HirExpressionKind::Component(c) = &self.hir[prop.expr().data].kind {
                        self.get_component_expression(*c, ctx, member_values)?
                    } else if let HirExpressionKind::Object { name, fields } = &self.hir[prop.expr().data].kind {
                        let ty = self.types.get_mapped_type(name).ok_or(CodegenError::new(
                            CodegenErrorKind::IRTypeNotRecognized(*name),
                            value.span,
                            ctx.file(),
                        ))?;
                        let field_values: Vec<Value> = fields
                            .iter()
                            .map(|field| {
                                if let Some(map) = member_values {
                                    if let HirExpressionKind::Identifier(var) =
                                        &self.hir[field.data].kind
                                    {
                                        if var.is_component() {
                                            return map.get(var).copied().unwrap_or_else(|| {
                                                self.lower_expression(*field, ctx)
                                                    .unwrap_or(Value::VOID)
                                            });
                                        }
                                    }
                                }
                                self.lower_expression(*field, ctx).unwrap_or(Value::VOID)
                            })
                            .collect();
                        ctx.struct_literal(ty, &field_values)
                    } else if let Some(map) = member_values {
                        if let HirExpressionKind::Identifier(var) = &self.hir[prop.expr().data].kind {
                            if var.is_component() {
                                map.get(var).copied().unwrap_or_else(|| {
                                    self.lower_expression(*prop.expr(), ctx).unwrap_or(Value::VOID)
                                })
                            } else {
                                self.lower_expression(*prop.expr(), ctx)?
                            }
                        } else {
                            self.lower_expression(*prop.expr(), ctx)?
                        }
                    } else {
                        self.lower_expression(*prop.expr(), ctx)?
                    };
                    prop_values[prop.index()] = val;
                    if let Some(id) = self.find_component_id(*name) {
                        let var_id = VariableId::component(id, prop.index() as u16);
                        current_member_values.insert(var_id, val);
                    }
                }
                // Fill defaults for omitted props with void; codegen relies on
                // explicit values elsewhere, so this only keeps indices aligned.
                all_values.extend(prop_values);
            }
            let mut ordered_children: Vec<_> = children.iter().collect();
            ordered_children.sort_by_key(|child| child.index());
            let mut child_member_values = current_member_values.clone();
            if let Some(parent_map) = member_values {
                for (key, value) in parent_map {
                    child_member_values.entry(*key).or_insert(*value);
                }
            }
            for child in ordered_children {
                let child_value = self.get_component_expression(
                    *child.expr(),
                    ctx,
                    Some(&child_member_values),
                )?;
                all_values.push(child_value);
            }
            (ty, all_values)
        };

        let mut cvb = ComponentValueBuilder::new(ctx, ty);
        for val in &all_values {
            cvb.add_argument(*val);
        }
        let comp_value = cvb.generate();

        Ok(comp_value)
    }
    pub(crate) fn get_type_of_component_expression(
        &self,
        expr: &HirComponentExpression,
        span: Span,
        file: FileId,
    ) -> Result<IRTypeId, CodegenError> {
        self.types
            .get_mapped_type(&expr.name)
            .ok_or(CodegenError::new(
                CodegenErrorKind::IRTypeNotRecognized(expr.name),
                span,
                file,
            ))
    }

    pub(crate) fn initialize_component(
        &mut self,
        id: DeclarationId<HirComponentDeclaration>,
        decl: &HirComponentDeclaration,
        props: &[ComponentMemberDeclaration],
        span: Span,
        ir: &mut SlynxIR,
        file: FileId,
    ) -> Result<(), CodegenError> {
        let ptr = *self
            .components
            .get(&id)
            .expect("Component should have been hoisted");
        let component_props = if let Some(viewer) = self.hir.view(decl.ty).is_component() {
            viewer.props().to_vec()
        } else {
            Vec::new()
        };
        let property_types = component_props
            .iter()
            .filter(|prop| !matches!(prop.property_kind, ComponentPropertyKind::View))
            .map(|prop| self.types.get_or_create_ir_type(prop.ty, span, ir, file))
            .collect::<Result<Vec<_>, CodegenError>>()?;

        // For each specialized child with a style usage, build the __child_init function
        // and record the parent property indices needed at instantiation time.
        let extra_vars: Vec<(VariableId, IRTypeId)> = Vec::new();

        let mut parent_prop_indices = Vec::with_capacity(extra_vars.len());
        for (_, _) in &extra_vars {
            let prop_idx = 0;
            parent_prop_indices.push(prop_idx);
        }

        let child_types: Vec<IRTypeId> = props
            .iter()
            .filter_map(|p| match p {
                ComponentMemberDeclaration::Child(c) => {
                    let comp_expr = &self.hir.store.component_expressions[c.data];
                    Some(self.get_type_of_component_expression(comp_expr, c.span, file))
                }
                _ => None,
            })
            .collect::<Result<_, CodegenError>>()?;

        let comp_ir_ty = self
            .types
            .get_mapped_type(&decl.ty)
            .ok_or(CodegenError::new(
                CodegenErrorKind::IRTypeNotRecognized(decl.ty),
                span,
                file,
            ))?;
        if let IRType::Component(cid) = *ir.types.get_type(comp_ir_ty) {
            let comp_ty = ir.types.get_component_type_mut(cid);
            for ty in &property_types {
                comp_ty.insert_field(*ty);
            }
            for ty in &child_types {
                comp_ty.insert_child(*ty);
            }
        }

        let view_funcs = self.generate_view_functions(id, props, ir, file, span)?;
        self.view_functions.insert(decl.ty, view_funcs.clone());
        let prepared_binds = self.prepare_reactive_binds(id, props, ir, file, span, &view_funcs)?;
        let mut builder = ComponentBuilder::new(ptr, ir);

        for (source, child, field, modifier, callbacks) in prepared_binds {
            builder.add_bind(source, child, field, modifier, callbacks);
        }

        builder.generate();

        Ok(())
    }

    pub(crate) fn ir_field_index(
        &self,
        props: &[ComponentMemberDeclaration],
        hir_index: usize,
    ) -> Option<usize> {
        let mut ir = 0usize;
        for prop in props {
            match prop {
                ComponentMemberDeclaration::Property { index, .. }
                | ComponentMemberDeclaration::Input { index, .. }
                    if *index == hir_index =>
                {
                    return Some(ir);
                }
                ComponentMemberDeclaration::Property { .. }
                | ComponentMemberDeclaration::Input { .. } => {
                    ir += 1;
                }
                _ => {}
            }
        }
        None
    }

    pub(crate) fn find_component_id(
        &self,
        term: TermId,
    ) -> Option<DeclarationId<HirComponentDeclaration>> {
        let view = self.hir.view(term).nominal();
        let comp = view.is_component()?;
        let owner = self.hir.types.get_component_owner(comp.data());
        let file = self.hir.get_file(owner);
        file.declarations
            .declarations
            .components
            .iter()
            .with_ids()
            .find(|(_, decl)| decl.ty == term)
            .map(|(id, _)| DeclarationId::new(owner, id))
    }

    pub(crate) fn find_component_props(
        &self,
        term: TermId,
    ) -> Vec<ComponentMemberDeclaration> {
        let view = match self.hir.view(term).nominal().is_component() {
            Some(v) => v,
            None => return Vec::new(),
        };
        let owner = self.hir.types.get_component_owner(view.data());
        let file = self.hir.get_file(owner);
        file.declarations
            .declarations
            .components
            .iter()
            .find(|decl| decl.ty == term)
            .map(|decl| decl.props.clone())
            .unwrap_or_default()
    }

    pub(crate) fn generate_view_functions(
        &mut self,
        id: DeclarationId<HirComponentDeclaration>,
        props: &[ComponentMemberDeclaration],
        ir: &mut SlynxIR,
        file: FileId,
        span: Span,
    ) -> Result<HashMap<usize, IRPointer<slynx_ir::Function, 1>>, CodegenError> {
        let decl = &self.hir.get_file(id.owner).declarations.components[id.term];
        let comp_name = self.hir.get_name(decl.name);
        let comp_ir_ty = self
            .types
            .get_mapped_type(&decl.ty)
            .ok_or(CodegenError::new(
                CodegenErrorKind::IRTypeNotRecognized(decl.ty),
                span,
                file,
            ))?;

        let views: Vec<(usize, SymbolPointer, Spanned<PoolId<HirExpression>>)> = props
            .iter()
            .filter_map(|prop| match prop {
                ComponentMemberDeclaration::View {
                    index, name, rhs, ..
                } => Some((*index, *name, *rhs)),
                _ => None,
            })
            .collect();

        let mut ptrs: HashMap<usize, (IRPointer<slynx_ir::Function, 1>, IRTypeId)> = HashMap::new();
        for (view_index, view_name, rhs) in &views {
            let view_ty_term = self.hir[rhs.data].ty.term;
            let view_ty = self.types.get_or_create_ir_type(view_ty_term, span, ir, file)?;
            let func_ptr = ir.create_function(
                &format!("{}_view{}", comp_name, self.hir.get_name(*view_name)),
                false,
            );
            ptrs.insert(*view_index, (func_ptr, view_ty));
        }

        for (view_index, _view_name, rhs) in &views {
            let (func_ptr, view_ty) = ptrs[view_index];
            let ptr_ty = ir.types.pointer_type(comp_ir_ty);
            let mut ctx = FunctionContext::new(ir.build_function(func_ptr), file);
            let entry = ctx.create_label("entry");
            ctx.block(entry)?;
            let arg_values = ctx
                .set_function_type(vec![ptr_ty], view_ty)
                .to_vec();
            let self_ptr = arg_values[0];

            for prop in props {
                match prop {
                    ComponentMemberDeclaration::Property { index, .. }
                    | ComponentMemberDeclaration::Input { index, .. } => {
                        if let Some(ir_idx) = self.ir_field_index(props, *index) {
                            let var_id = VariableId::component(id, *index as u16);
                            let val = ctx.get_field(self_ptr, ir_idx as u16);
                            ctx.add_variable(var_id, val);
                        }
                    }
                    ComponentMemberDeclaration::View { index: vidx, .. } => {
                        if *vidx != *view_index {
                            if let Some((vf, vty)) = ptrs.get(vidx) {
                                let var_id = VariableId::component(id, *vidx as u16);
                                let val = ctx.call(*vf, &[self_ptr], *vty);
                                ctx.add_variable(var_id, val);
                            }
                        }
                    }
                    _ => {}
                }
            }

            let val = self.lower_expression(*rhs, &mut ctx)?;
            ctx.ret(val);
            ctx.finish();
        }

        Ok(ptrs.into_iter().map(|(k, (p, _))| (k, p)).collect())
    }

    pub(crate) fn prepare_reactive_binds(
        &mut self,
        id: DeclarationId<HirComponentDeclaration>,
        props: &[ComponentMemberDeclaration],
        ir: &mut SlynxIR,
        file: FileId,
        span: Span,
        view_funcs: &HashMap<usize, IRPointer<slynx_ir::Function, 1>>,
    ) -> Result<
        Vec<(
            u16,
            u16,
            u16,
            Option<IRPointer<slynx_ir::Function, 1>>,
            Vec<IRPointer<slynx_ir::Function, 1>>,
        )>,
        CodegenError,
    > {
        let Some(graph) = self.hir.component_graphs.get(&id) else {
            return Ok(Vec::new());
        };

        let mut field_expr: HashMap<(usize, usize), Spanned<PoolId<HirExpression>>> = HashMap::new();
        let mut child_terms: Vec<TermId> = Vec::new();
        let mut child_idx = 0usize;
        for member in props {
            if let ComponentMemberDeclaration::Child(c) = member {
                let comp_expr = &self.hir.store.component_expressions[c.data];
                for p in &comp_expr.properties {
                    field_expr.insert((child_idx, p.index()), *p.expr());
                }
                child_terms.push(comp_expr.name);
                child_idx += 1;
            }
        }

        fn expand_view_sources(
            graph: &slynx_hir::reactive_graph::ReactiveGraph,
            view_index: usize,
        ) -> Vec<usize> {
            let mut out = Vec::new();
            let view_node = graph
                .graph
                .node_indices()
                .find(|n| matches!(graph.graph[*n], DagNode::View { index } if index == view_index));
            let Some(view_node) = view_node else {
                return out;
            };
            for edge in graph.graph.edges_directed(view_node, petgraph::Direction::Incoming) {
                match graph.graph[edge.source()] {
                    DagNode::Property { index } | DagNode::Input { index } => out.push(index),
                    DagNode::View { index } => out.extend(expand_view_sources(graph, index)),
                    _ => {}
                }
            }
            out
        }

        let mut update_funcs: HashMap<(usize, usize), IRPointer<slynx_ir::Function, 1>> = HashMap::new();
        let mut prepared: Vec<(
            u16,
            u16,
            u16,
            Option<IRPointer<slynx_ir::Function, 1>>,
            Vec<IRPointer<slynx_ir::Function, 1>>,
        )> = Vec::new();

        for node in graph.graph.node_indices() {
            let DagNode::ChildField { child, field } = graph.graph[node] else {
                continue;
            };
            let Some(expr) = field_expr.get(& (child, field)) else {
                continue;
            };

            let mut triggers: Vec<usize> = Vec::new();
            for edge in graph.graph.edges_directed(node, petgraph::Direction::Incoming) {
                match graph.graph[edge.source()] {
                    DagNode::Property { index } | DagNode::Input { index } => triggers.push(index),
                    DagNode::View { index } => triggers.extend(expand_view_sources(&graph, index)),
                    _ => {}
                }
            }
            triggers.sort();
            triggers.dedup();
            if triggers.is_empty() {
                continue;
            }

            let child_props = child_terms
                .get(child)
                .copied()
                .map(|t| self.find_component_props(t))
                .unwrap_or_default();
            let child_field_ir = self.ir_field_index(&child_props, field).unwrap_or(field);

            let is_simple = triggers.len() == 1
                && matches!(self.hir[expr.data].kind, HirExpressionKind::Identifier(var) if var.is_component() && var.index as usize == triggers[0]);

            let update_func = if is_simple {
                None
            } else {
                if !update_funcs.contains_key(&(child, field)) {
                    let uf = self.generate_update_function(id, props, *expr, span, ir, file, view_funcs)?;
                    update_funcs.insert((child, field), uf);
                }
                Some(update_funcs[&(child, field)])
            };

            let cb_defs = child_terms
                .get(child)
                .copied()
                .map(|t| {
                    self.hir
                        .types
                        .callbacks
                        .property_callbacks_of(t, field)
                })
                .unwrap_or_default();
            let cb_funcs: Vec<IRPointer<slynx_ir::Function, 1>> = cb_defs
                .iter()
                .filter_map(|cb| self.functions.get(&cb.function).copied())
                .collect();

            for trigger in triggers {
                if let Some(source_ir) = self.ir_field_index(props, trigger) {
                    prepared.push((
                        source_ir as u16,
                        child as u16,
                        child_field_ir as u16,
                        update_func,
                        cb_funcs.clone(),
                    ));
                }
            }
        }

        Ok(prepared)
    }

    pub(crate) fn generate_update_function(
        &mut self,
        id: DeclarationId<HirComponentDeclaration>,
        props: &[ComponentMemberDeclaration],
        expr: Spanned<PoolId<HirExpression>>,
        span: Span,
        ir: &mut SlynxIR,
        file: FileId,
        view_funcs: &HashMap<usize, IRPointer<slynx_ir::Function, 1>>,
    ) -> Result<IRPointer<slynx_ir::Function, 1>, CodegenError> {
        let decl = &self.hir.get_file(id.owner).declarations.components[id.term];
        let comp_name = self.hir.get_name(decl.name);
        let comp_ir_ty = self
            .types
            .get_mapped_type(&decl.ty)
            .ok_or(CodegenError::new(
                CodegenErrorKind::IRTypeNotRecognized(decl.ty),
                span,
                file,
            ))?;
        let ret_ty = self.types.get_or_create_ir_type(self.hir[expr.data].ty.term, span, ir, file)?;
        let ptr_ty = ir.types.pointer_type(comp_ir_ty);

        let mut view_ir_types: HashMap<usize, IRTypeId> = HashMap::new();
        for prop in props {
            if let ComponentMemberDeclaration::View { index: vidx, rhs, .. } = prop {
                let view_ty_term = self.hir[rhs.data].ty.term;
                let view_ty = self.types.get_or_create_ir_type(view_ty_term, span, ir, file)?;
                view_ir_types.insert(*vidx, view_ty);
            }
        }

        let func_ptr = ir.create_function(&format!("{}_bind", comp_name), false);
        let mut ctx = FunctionContext::new(ir.build_function(func_ptr), file);
        let entry = ctx.create_label("entry");
        ctx.block(entry)?;
        let arg_values = ctx
            .set_function_type(vec![ptr_ty], ret_ty)
            .to_vec();
        let self_ptr = arg_values[0];

        for prop in props {
            match prop {
                ComponentMemberDeclaration::Property { index, .. }
                | ComponentMemberDeclaration::Input { index, .. } => {
                    if let Some(ir_idx) = self.ir_field_index(props, *index) {
                        let var_id = VariableId::component(id, *index as u16);
                        let val = ctx.get_field(self_ptr, ir_idx as u16);
                        ctx.add_variable(var_id, val);
                    }
                }
                ComponentMemberDeclaration::View { index: vidx, .. } => {
                    if let Some(vf) = view_funcs.get(vidx) {
                        if let Some(view_ty) = view_ir_types.get(vidx) {
                            let var_id = VariableId::component(id, *vidx as u16);
                            let val = ctx.call(*vf, &[self_ptr], *view_ty);
                            ctx.add_variable(var_id, val);
                        }
                    }
                }
                _ => {}
            }
        }

        let val = self.lower_expression(expr, &mut ctx)?;
        ctx.ret(val);
        ctx.finish();
        Ok(func_ptr)
    }
}
