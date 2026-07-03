use common::{
    Spanned,
    pool::{DedupPoolId, PoolId},
};
use slynx_hir::{
    ComponentMemberDeclaration, DeclarationId, HirComponentDeclaration, HirComponentExpression,
    HirExpression, HirExpressionKind, HirStyleUsage, HirType, SlynxHir, VariableId,
};
use slynx_ir::{
    ComponentBuilder, ComponentValueBuilder, Function, IRPointer, IRTypeId, SlynxIR, Value,
};

use crate::{ChildInitWork, Codegen, CodegenError, TypeId, functions::FunctionContext};

fn collect_var_ids_from_expr(
    hir: &SlynxHir,
    expr: &Spanned<PoolId<HirExpression>>,
    out: &mut Vec<VariableId>,
) {
    let expression = &hir[expr.data];
    match &expression.kind {
        HirExpressionKind::Identifier(id) if !out.contains(id) => {
            out.push(*id);
        }

        HirExpressionKind::Binary { lhs, rhs, .. } => {
            collect_var_ids_from_expr(hir, lhs, out);
            collect_var_ids_from_expr(hir, rhs, out);
        }
        HirExpressionKind::Tuple(items) => {
            for item in items {
                collect_var_ids_from_expr(hir, item, out);
            }
        }
        HirExpressionKind::FieldAccess { expr, .. } => {
            collect_var_ids_from_expr(hir, expr, out);
        }
        _ => {}
    }
}

/// Map a VariableId to its property index in the component type.
/// Looks up the variable name from the HIR symbols resolver,
/// then finds the property with that name in the component's props list.
fn var_id_to_prop_index(
    hir: &SlynxHir,
    comp_ty: DedupPoolId<HirType>,
    var_id: VariableId,
) -> Option<usize> {
    if let Some(component) = hir.view(comp_ty).is_component() {
        None
        //component.props().iter().position(|p| p.name() == name)
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
        let style =
            self.styles
                .get(&style_usage.style)
                .ok_or(CodegenError::DeclarationNotRecognized(
                    style_usage.style.into(),
                ))?;
        Ok(StyleApplyData {
            init_func: style.init_func,
            apply_func: style.apply_func,
            struct_ty: style.struct_ty,
        })
    }

    pub(crate) fn get_component_expression(
        &mut self,
        value: Spanned<PoolId<HirComponentExpression>>,
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<(Value, Option<StyleApplyData>), CodegenError> {
        let HirComponentExpression {
            name,
            properties,
            children,
            ..
        } = &hir[value.data];
        let (ty, style_usage, all_values) = {
            let ty = self
                .get_mapped_type(name)
                .ok_or(CodegenError::IRTypeNotRecognized(*name))?;

            let mut all_values = Vec::new();
            if let Some(viewer) = hir.view(*name).is_component() {
                let props = viewer.props();
                let num_props = props.len();
                let mut prop_values = vec![Value::VOID; num_props];
                for prop in properties {
                    let val = self.lower_expression(*prop.expr(), hir, ctx)?;
                    prop_values[prop.index()] = val;
                }
                all_values.extend(prop_values);
            }
            for child in children {
                let (child_value, _) = self.get_component_expression(*child, hir, ctx)?;
                all_values.push(child_value);
            }
            (ty, None, all_values)
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
        self.get_mapped_type(&expr.name)
            .ok_or(CodegenError::IRTypeNotRecognized(expr.name))
    }

    pub(crate) fn get_usage_args(
        &mut self,
        usage: &HirStyleUsage,
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<Vec<Value>, CodegenError> {
        let mut out = Vec::with_capacity(usage.params.len());
        for param in &usage.params {
            let value = self.lower_expression(*param, hir, ctx)?;
            out.push(value);
        }
        Ok(out)
    }

    fn get_component_initcall(&mut self, ty: TypeId) -> Result<(), CodegenError> {
        let initcall_info = ChildInitWork {
            children_type: Vec::new(),
            children_index: Vec::new(),
            init_func: IRPointer::null(),
            parent_prop_indices: Vec::new(),
        };

        self.component_child_inits
            .entry(ty)
            .or_default()
            .push(initcall_info);
        Ok(())
    }

    pub(crate) fn initialize_component(
        &mut self,
        id: DeclarationId<HirComponentDeclaration>,
        decl: &HirComponentDeclaration,
        props: &[ComponentMemberDeclaration],
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let ptr = *self
            .components
            .get(&id)
            .expect("Component should have been hoisted");
        let component_props = if let Some(viewer) = hir.view(decl.ty).is_component() {
            viewer.props().to_vec()
        } else {
            Vec::new()
        };
        let property_types = component_props
            .iter()
            .map(|prop| self.get_or_create_ir_type(prop, hir, ir))
            .collect::<Result<Vec<_>, CodegenError>>()?;

        // For each specialized child with a style usage, build the __child_init function
        // and record the parent property indices needed at instantiation time.
        let extra_vars: Vec<(VariableId, IRTypeId)> = Vec::new();

        let mut parent_prop_indices = Vec::with_capacity(extra_vars.len());
        for (var_id, _) in &extra_vars {
            let prop_idx = var_id_to_prop_index(hir, decl.ty, *var_id).unwrap_or(0);
            parent_prop_indices.push(prop_idx);
        }
        self.get_component_initcall(decl.ty)?;

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
