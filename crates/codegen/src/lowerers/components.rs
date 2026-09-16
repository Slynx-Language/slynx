use common::{Spanned, pool::PoolId};
use slynx_hir::{
    ComponentMemberDeclaration, DeclarationId, HirComponentDeclaration, HirComponentExpression,
    HirStyleUsage, VariableId,
};
use slynx_ir::{ComponentBuilder, ComponentValueBuilder, IRTypeId, SlynxIR, Value};

use crate::{
    CodegenError,
    lowerers::{LoweringState, functions::FunctionContext},
};

impl<'a> LoweringState<'a> {
    pub(crate) fn get_component_expression(
        &mut self,
        value: Spanned<PoolId<HirComponentExpression>>,
        ctx: &mut FunctionContext,
    ) -> Result<Value, CodegenError> {
        let HirComponentExpression {
            name,
            properties,
            children,
            ..
        } = &self.hir[value.data];
        let (ty, all_values) = {
            let ty = self
                .types
                .get_mapped_type(name)
                .ok_or(CodegenError::IRTypeNotRecognized(*name))?;

            let mut all_values = Vec::new();
            if let Some(viewer) = self.hir.view(*name).is_component() {
                let props = viewer.props();
                let num_props = props.len();
                let mut prop_values = vec![Value::VOID; num_props];
                for prop in properties {
                    let val = self.lower_expression(*prop.expr(), ctx)?;
                    prop_values[prop.index()] = val;
                }
                all_values.extend(prop_values);
            }
            for child in children {
                let child_value = self.get_component_expression(*child, ctx)?;
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
        _ir: &SlynxIR,
    ) -> Result<IRTypeId, CodegenError> {
        self.types
            .get_mapped_type(&expr.name)
            .ok_or(CodegenError::IRTypeNotRecognized(expr.name))
    }

    pub(crate) fn get_usage_args(
        &mut self,
        usage: &HirStyleUsage,
        ctx: &mut FunctionContext,
    ) -> Result<Vec<Value>, CodegenError> {
        let mut out = Vec::with_capacity(usage.params.len());
        for param in &usage.params {
            let value = self.lower_expression(*param, ctx)?;
            out.push(value);
        }
        Ok(out)
    }

    pub(crate) fn initialize_component(
        &mut self,
        id: DeclarationId<HirComponentDeclaration>,
        decl: &HirComponentDeclaration,
        props: &[ComponentMemberDeclaration],
        ir: &mut SlynxIR,
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
            .map(|prop| self.types.get_or_create_ir_type(*prop, ir))
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
                    Some(self.get_type_of_component_expression(comp_expr, ir))
                }
                ComponentMemberDeclaration::Property { .. } => None,
            })
            .collect::<Result<_, CodegenError>>()?;

        let mut builder = ComponentBuilder::new(ptr, ir);

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

        builder.generate();

        Ok(())
    }
}
