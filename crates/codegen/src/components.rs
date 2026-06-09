use slynx_hir::{
    ComponentMemberDeclaration, HirComponentExpression, HirDeclaration, HirExpression,
    HirExpressionKind, HirSpecializedComponentExpression, HirStyleUsage, HirType, SlynxHir, TypeId,
    VariableId,
};
use slynx_ir::{
    ComponentBuilder, ComponentValueBuilder, Function, IRPointer, IRTypeId, SlynxIR, Value,
};

use crate::{ChildInitWork, Codegen, CodegenError, functions::FunctionContext};

fn collect_var_ids_from_expr(expr: &HirExpression, out: &mut Vec<VariableId>) {
    match &expr.kind {
        HirExpressionKind::Identifier(id) if !out.contains(id) => {
            out.push(*id);
        }

        HirExpressionKind::Binary { lhs, rhs, .. } => {
            collect_var_ids_from_expr(lhs, out);
            collect_var_ids_from_expr(rhs, out);
        }
        HirExpressionKind::Tuple(items) => {
            for item in items {
                collect_var_ids_from_expr(item, out);
            }
        }
        HirExpressionKind::FieldAccess { expr, .. } => {
            collect_var_ids_from_expr(expr, out);
        }
        _ => {}
    }
}

/// Map a VariableId to its property index in the component type.
/// Looks up the variable name from the HIR symbols resolver,
/// then finds the property with that name in the component's props list.
fn var_id_to_prop_index(hir: &SlynxHir, comp_ty: &TypeId, var_id: VariableId) -> Option<usize> {
    let name = *hir.symbols_resolver.variables().get(&var_id)?.value();
    if let HirType::Component { props } = &*hir.get_type(comp_ty) {
        props.iter().position(|p| p.name() == name)
    } else {
        None
    }
}

pub struct StyleApplyData {
    pub init_func: IRPointer<Function, 1>,
    pub apply_func: IRPointer<Function, 1>,
    pub struct_ty: IRTypeId,
}

impl Codegen {
    fn get_style_application(
        &self,
        style_usage: &HirStyleUsage,
    ) -> Result<StyleApplyData, CodegenError> {
        let style = self
            .styles
            .get(&style_usage.style)
            .ok_or(CodegenError::DeclarationNotRecognized(style_usage.style))?;
        Ok(StyleApplyData {
            init_func: style.init_func,
            apply_func: style.apply_func,
            struct_ty: style.struct_ty,
        })
    }

    pub(crate) fn get_component_expression(
        &mut self,
        value: &HirComponentExpression,
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<(Value, Option<StyleApplyData>), CodegenError> {
        let (ty, style_usage, all_values) = match value {
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Text {
                text,
                style,
            }) => {
                let ty = ctx.ir().specialized_text_type();
                let text_value = self.lower_expression(text, hir, ctx)?;
                (ty, style.as_ref(), vec![text_value])
            }
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Div {
                children,
                style,
            }) => {
                let ty = ctx.ir().specialized_div_type();
                let mut values = Vec::with_capacity(children.len());
                for child in children {
                    let (child_value, _) = self.get_component_expression(child, hir, ctx)?;
                    values.push(child_value);
                }
                (ty, style.as_ref(), values)
            }
            HirComponentExpression::Normal {
                name,
                properties,
                children,
                ..
            } => {
                let ty = self
                    .get_mapped_type(name)
                    .ok_or(CodegenError::IRTypeNotRecognized(*name))?;

                let mut all_values = Vec::new();
                if let HirType::Component { props } = &*hir.get_type(name) {
                    let num_props = props.len();
                    let mut prop_values = vec![Value::VOID; num_props];
                    for prop in properties {
                        let val = self.lower_expression(prop.expr(), hir, ctx)?;
                        prop_values[prop.index()] = val;
                    }
                    all_values.extend(prop_values);
                }
                for child in children {
                    let (child_value, _) = self.get_component_expression(child, hir, ctx)?;
                    all_values.push(child_value);
                }
                (ty, None, all_values)
            }
        };

        let mut cvb = ComponentValueBuilder::new(ctx, ty);
        for val in &all_values {
            cvb.add_argument(*val);
        }
        let comp_value = cvb.generate();

        let style_application = if let Some(style) = style_usage {
            Some(self.get_style_application(style)?)
        } else {
            None
        };
        Ok((comp_value, style_application))
    }
    pub(crate) fn get_type_of_component_expression(
        &self,
        expr: &HirComponentExpression,
        ir: &SlynxIR,
    ) -> Result<IRTypeId, CodegenError> {
        match expr {
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Text {
                ..
            }) => Ok(ir.specialized_text_type()),
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Div {
                ..
            }) => Ok(ir.specialized_div_type()),
            HirComponentExpression::Normal { name, .. } => self
                .get_mapped_type(name)
                .ok_or(CodegenError::IRTypeNotRecognized(*name)),
        }
    }

    pub(crate) fn get_usage_args(
        &mut self,
        usage: &HirStyleUsage,
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<Vec<Value>, CodegenError> {
        let mut out = Vec::with_capacity(usage.params.len());
        for param in &usage.params {
            let value = self.lower_expression(param, hir, ctx)?;
            out.push(value);
        }
        Ok(out)
    }

    fn build_child_init(
        &mut self,
        parent_name: &str,
        children: &[&HirSpecializedComponentExpression],
        parent_prop_vars: &[(VariableId, IRTypeId)],
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<IRPointer<Function, 1>, CodegenError> {
        let void_ty = ir.void_type();
        let mut children_ty: Vec<_> = children
            .iter()
            .map(|c| match c {
                HirSpecializedComponentExpression::Text { .. } => ir.specialized_text_type(),
                HirSpecializedComponentExpression::Div { .. } => ir.specialized_div_type(),
            })
            .collect();

        let fptr = ir.create_function(&format!("_init_{parent_name}"));
        let builder = ir.build_function(fptr);
        let mut ctx = FunctionContext::new(builder);
        let entry = ctx.create_label("entry");
        ctx.goto(entry).unwrap();
        children_ty.extend(parent_prop_vars.iter().map(|(_, ty)| *ty));
        let args = ctx.set_function_type(children_ty, void_ty).to_vec();

        for (i, (var_id, _)) in parent_prop_vars.iter().enumerate() {
            ctx.add_variable(*var_id, args[children.len() + i]);
        }
        for (child, child_value) in children.iter().zip(&args) {
            match child {
                HirSpecializedComponentExpression::Text { text, style } => {
                    if let Some(style_usage) = style {
                        let style_data = self.get_style_application(style_usage)?;
                        let param_values = self.get_usage_args(style_usage, hir, &mut ctx)?;
                        let struct_val =
                            ctx.call(style_data.init_func, &param_values, style_data.struct_ty);
                        ctx.call(style_data.apply_func, &[*child_value, struct_val], void_ty);
                    }
                    let text_value = self.lower_expression(text, hir, &mut ctx)?;
                    ctx.set_field(*child_value, 0, text_value);
                }
                HirSpecializedComponentExpression::Div { children, style } => {
                    if let Some(style_usage) = style {
                        let style_data = self.get_style_application(style_usage)?;
                        let param_values = self.get_usage_args(style_usage, hir, &mut ctx)?;
                        let struct_val =
                            ctx.call(style_data.init_func, &param_values, style_data.struct_ty);
                        ctx.call(style_data.apply_func, &[*child_value, struct_val], void_ty);
                    }

                    for (child_index, child_expr) in children.iter().enumerate() {
                        let (child_val, child_style) =
                            self.get_component_expression(child_expr, hir, &mut ctx)?;

                        if let Some(ref child_style_data) = child_style {
                            let child_style_usage = match child_expr {
                                HirComponentExpression::Specialized(
                                    HirSpecializedComponentExpression::Text {
                                        style: Some(usage),
                                        ..
                                    },
                                ) => usage,
                                HirComponentExpression::Specialized(
                                    HirSpecializedComponentExpression::Div {
                                        style: Some(usage),
                                        ..
                                    },
                                ) => usage,
                                _ => unreachable!(),
                            };
                            let param_values =
                                self.get_usage_args(child_style_usage, hir, &mut ctx)?;
                            let struct_val = ctx.call(
                                child_style_data.init_func,
                                &param_values,
                                child_style_data.struct_ty,
                            );
                            ctx.call(
                                child_style_data.apply_func,
                                &[child_val, struct_val],
                                void_ty,
                            );
                        }

                        ctx.set_field(*child_value, child_index as u16, child_val);
                    }
                }
            }
        }

        ctx.ret(Value::VOID);
        ctx.finish();
        Ok(fptr)
    }

    fn get_specialized_type(
        spec_component: &HirSpecializedComponentExpression,
        ir: &SlynxIR,
    ) -> IRTypeId {
        match spec_component {
            HirSpecializedComponentExpression::Text { .. } => ir.specialized_text_type(),
            HirSpecializedComponentExpression::Div { .. } => ir.specialized_div_type(),
        }
    }

    fn get_component_initcall(
        &mut self,
        ty: TypeId,
        parent_name: &str,
        props: &[ComponentMemberDeclaration],
        extra_vars: &[(VariableId, IRTypeId)],
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let mut initcall_info = ChildInitWork {
            children_type: Vec::new(),
            children_index: Vec::new(),
            init_func: IRPointer::null(),
            parent_prop_indices: Vec::new(),
        };

        let mut spec_children = Vec::new();
        for (child_index, prop) in props.iter().enumerate() {
            if let ComponentMemberDeclaration::Child(HirComponentExpression::Specialized(spec)) =
                prop
            {
                let ty = Self::get_specialized_type(spec, ir);
                spec_children.push(spec);
                initcall_info.children_index.push(child_index);
                initcall_info.children_type.push(ty);
            }
        }
        let init_func = self.build_child_init(parent_name, &spec_children, extra_vars, hir, ir)?;
        initcall_info.init_func = init_func;
        self.component_child_inits
            .entry(ty)
            .or_default()
            .push(initcall_info);
        Ok(())
    }

    pub(crate) fn initialize_component(
        &mut self,
        decl: &HirDeclaration,
        props: &[ComponentMemberDeclaration],
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let ptr = *self
            .components
            .get(&decl.id)
            .expect("Component should have been hoisted");
        let component_props = if let HirType::Component { props } = &*hir.get_type(&decl.ty) {
            props.clone()
        } else {
            Vec::new()
        };
        let property_types = component_props
            .iter()
            .map(|prop| self.get_or_create_ir_type(prop.prop_type(), hir, ir))
            .collect::<Result<Vec<_>, CodegenError>>()?;

        let parent_name = hir.get_declaration_name(decl.id);

        // For each specialized child with a style usage, build the __child_init function
        // and record the parent property indices needed at instantiation time.
        let mut extra_vars: Vec<(VariableId, IRTypeId)> = Vec::new();
        for prop in props.iter() {
            if let ComponentMemberDeclaration::Child(HirComponentExpression::Specialized(
                child_spec,
            )) = prop
            {
                let style_usage = match child_spec {
                    HirSpecializedComponentExpression::Text { style, .. } => style,
                    HirSpecializedComponentExpression::Div { style, .. } => style,
                };

                if let Some(usage) = style_usage {
                    for param in &usage.params {
                        let mut collected = Vec::new();
                        collect_var_ids_from_expr(param, &mut collected);
                        for var_id in collected {
                            if !extra_vars.iter().any(|(v, _)| *v == var_id) {
                                let ir_ty = self.get_or_create_ir_type(&param.ty, hir, ir)?;
                                extra_vars.push((var_id, ir_ty));
                            }
                        }
                    }
                }
            }
        }
        let mut parent_prop_indices = Vec::with_capacity(extra_vars.len());
        for (var_id, _) in &extra_vars {
            let prop_idx = var_id_to_prop_index(hir, &decl.ty, *var_id).unwrap_or(0);
            parent_prop_indices.push(prop_idx);
        }
        self.get_component_initcall(decl.ty, parent_name, props, &extra_vars, hir, ir)?;

        let child_types: Vec<IRTypeId> = props
            .iter()
            .filter_map(|p| match p {
                ComponentMemberDeclaration::Child(c) => {
                    Some(self.get_type_of_component_expression(c, ir))
                }
                ComponentMemberDeclaration::Property { .. } => None,
            })
            .collect::<Result<_, CodegenError>>()?;

        let mut builder = ComponentBuilder::new(ptr, ir);

        let child_values: Vec<Value> = child_types
            .iter()
            .map(|ty| builder.emit_child_component(*ty))
            .collect();

        for child_ty in &child_types {
            builder.add_child(*child_ty);
        }
        for ty in &property_types {
            builder.add_field(*ty);
        }

        let mut prop_idx_to_child_val: Vec<Option<usize>> = vec![None; props.len()];
        let mut child_only_idx = 0;
        for (prop_idx, prop) in props.iter().enumerate() {
            if matches!(prop, ComponentMemberDeclaration::Child(_)) {
                prop_idx_to_child_val[prop_idx] = Some(child_only_idx);
                child_only_idx += 1;
            }
        }

        if let Some(child_inits) = self.component_child_inits.get(&decl.ty) {
            for init in child_inits {
                let mut args = init
                    .parent_prop_indices
                    .iter()
                    .map(|index| builder.emit_arg(*index as u32))
                    .collect::<Vec<_>>();
                for index in init.children_index.iter() {
                    let child_only_idx = prop_idx_to_child_val[*index]
                        .expect("child_index should map to a child-only index");
                    let child = child_values[child_only_idx];
                    args.push(child);
                }
                builder.add_initial_call(init.init_func, args);
            }
        }
        builder.generate();

        Ok(())
    }
}
