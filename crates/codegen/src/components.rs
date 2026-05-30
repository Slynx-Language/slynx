use slynx_hir::{
    ComponentMemberDeclaration, HirComponentExpression, HirDeclaration,
    HirExpression, HirExpressionKind, HirSpecializedComponentExpression, HirStyleUsage,
    HirType, SlynxHir, TypeId, VariableId,
};
use slynx_ir::{
    ComponentBuilder, ComponentValueBuilder, Function, IRPointer, IRTypeId,
    Opcode, SlynxIR, Value,
};

use crate::{Codegen, CodegenError, ChildInitWork, functions::FunctionContext};

fn collect_var_ids_from_expr(expr: &HirExpression, out: &mut Vec<VariableId>) {
    match &expr.kind {
        HirExpressionKind::Identifier(id) => {
            if !out.contains(id) {
                out.push(*id);
            }
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
fn var_id_to_prop_index(
    hir: &SlynxHir,
    comp_ty: &TypeId,
    var_id: VariableId,
) -> Option<usize> {
    let name = hir
        .modules
        .symbols_resolver
        .variables()
        .get(&var_id)
        .copied()?;
    if let HirType::Component { props } = hir.get_type(comp_ty) {
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
        let (decl_id, ty, style_usage, all_values) = match value {
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Text {
                text,
                style,
            }) => {
                let ty = ctx.ir().specialized_text_type();
                let text_value = self.lower_expression(text, hir, ctx)?;
                (None, ty, style.as_ref(), vec![text_value])
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
                (None, ty, style.as_ref(), values)
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
                if let HirType::Component { props } = hir.get_type(name) {
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
                (Some(*name), ty, None, all_values)
            }
        };

        let mut cvb = ComponentValueBuilder::new(ctx, ty);
        for val in &all_values {
            cvb.add_argument(*val);
        }
        let comp_value = cvb.generate();

        // Emit child initcalls for default children in the function body,
        // where actual property values are available as operands.
        if let Some(decl_id) = decl_id {
            if let Some(work_items) = self.component_child_inits.get(&decl_id) {
                for work in work_items {
                    let child_val = ctx.emit(Opcode::Component, smallvec::smallvec![], work.child_type);
                    let mut args = vec![child_val];
                    for &prop_idx in &work.parent_prop_indices {
                        args.push(all_values[prop_idx]);
                    }
                    ctx.initcall(work.init_func, &args);
                }
            }
        }

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
        child: &HirSpecializedComponentExpression,
        parent_prop_vars: &[(VariableId, IRTypeId)],
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<IRPointer<Function, 1>, CodegenError> {
        let child_ty = match child {
            HirSpecializedComponentExpression::Text { .. } => ir.specialized_text_type(),
            HirSpecializedComponentExpression::Div { .. } => ir.specialized_div_type(),
        };

        let void_ty = ir.void_type();
        let fptr = ir.create_function("__child_init");

        let builder = ir.build_function(fptr);
        let mut ctx = FunctionContext::new(builder);
        let entry = ctx.create_label("entry");
        ctx.switch_to_block(entry).unwrap();

        let mut arg_types = vec![child_ty];
        arg_types.extend(parent_prop_vars.iter().map(|(_, ty)| *ty));
        let args = ctx.set_function_type(arg_types, void_ty).to_vec();

        let child_value = args[0];
        for (i, (var_id, _)) in parent_prop_vars.iter().enumerate() {
            ctx.add_variable(*var_id, args[i + 1]);
        }

        match child {
            HirSpecializedComponentExpression::Text { text, style } => {
                if let Some(style_usage) = style {
                    let style_data = self.get_style_application(style_usage)?;
                    let param_values = self.get_usage_args(style_usage, hir, &mut ctx)?;
                    let struct_val =
                        ctx.call(style_data.init_func, &param_values, style_data.struct_ty);
                    ctx.call(style_data.apply_func, &[child_value, struct_val], void_ty);
                }
                let text_value = self.lower_expression(text, hir, &mut ctx)?;
                ctx.set_field(child_value, 0, text_value);
            }
            HirSpecializedComponentExpression::Div { children, style } => {
                if let Some(style_usage) = style {
                    let style_data = self.get_style_application(style_usage)?;
                    let param_values = self.get_usage_args(style_usage, hir, &mut ctx)?;
                    let struct_val =
                        ctx.call(style_data.init_func, &param_values, style_data.struct_ty);
                    ctx.call(style_data.apply_func, &[child_value, struct_val], void_ty);
                }
                for (i, cexpr) in children.iter().enumerate() {
                    let (child_val, _) = self.get_component_expression(cexpr, hir, &mut ctx)?;
                    ctx.set_field(child_value, i as u16, child_val);
                }
            }
        }

        ctx.ret(Value::VOID);
        ctx.finish();
        Ok(fptr)
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
        let property_types = if let HirType::Component { props } = hir.get_type(&decl.ty) {
            props
                .iter()
                .map(|prop| self.get_or_create_ir_type(prop.prop_type(), hir, ir))
                .collect::<Result<Vec<_>, CodegenError>>()?
        } else {
            Vec::new()
        };

        // For each specialized child with a style usage, build the __child_init function
        // and record the parent property indices needed at instantiation time.
        for prop in props {
            if let ComponentMemberDeclaration::Child(HirComponentExpression::Specialized(
                child_spec,
            )) = prop
            {
                let style_usage = match child_spec {
                    HirSpecializedComponentExpression::Text { style, .. } => style,
                    HirSpecializedComponentExpression::Div { style, .. } => style,
                };
                let mut extra_vars: Vec<(VariableId, IRTypeId)> = Vec::new();
                if let Some(usage) = style_usage {
                    for param in &usage.params {
                        let mut collected = Vec::new();
                        collect_var_ids_from_expr(param, &mut collected);
                        for var_id in collected {
                            if !extra_vars.iter().any(|(v, _)| *v == var_id) {
                                let ir_ty =
                                    self.get_or_create_ir_type(&param.ty, hir, ir)?;
                                extra_vars.push((var_id, ir_ty));
                            }
                        }
                    }
                }

                // Map VariableIds to property indices for use at instantiation.
                let mut parent_prop_indices = Vec::with_capacity(extra_vars.len());
                for (var_id, _) in &extra_vars {
                    // This should always succeed for valid HIR.
                    let prop_idx = var_id_to_prop_index(hir, &decl.ty, *var_id).unwrap_or(0);
                    parent_prop_indices.push(prop_idx);
                }

                let init_func = self.build_child_init(child_spec, &extra_vars, hir, ir)?;
                let child_type = match child_spec {
                    HirSpecializedComponentExpression::Text { .. } => ir.specialized_text_type(),
                    HirSpecializedComponentExpression::Div { .. } => ir.specialized_div_type(),
                };

                self.component_child_inits
                    .entry(decl.ty)
                    .or_default()
                    .push(ChildInitWork {
                        child_type,
                        init_func,
                        parent_prop_indices,
                    });
            }
        }

        // Phase 1: collect child types
        let child_types: Vec<IRTypeId> = props
            .iter()
            .filter_map(|p| match p {
                ComponentMemberDeclaration::Child(c) => {
                    Some(self.get_type_of_component_expression(c, ir))
                }
                ComponentMemberDeclaration::Property { .. } => None,
            })
            .collect::<Result<Vec<_>, CodegenError>>()?;

        // Phase 2: use ComponentBuilder (no initcalls in preamble)
        let mut builder = ComponentBuilder::new(ptr, ir);

        for child_ty in &child_types {
            builder.add_child(*child_ty);
        }
        for ty in &property_types {
            builder.add_field(*ty);
        }

        builder.generate();

        Ok(())
    }
}
