use std::collections::HashSet;

use slynx_hir::{
    HirDeclaration, HirDeclarationKind, HirStyleBlockKind, HirStyleStatement, HirStyleUsage,
    HirType, SlynxHir, StylesDefinition, TypeId,
};
use slynx_ir::{Function, IRPointer, IRType, IRTypeId, SlynxIR, StyleProperty, Value};

use crate::{Codegen, CodegenError, functions::FunctionContext};

pub struct StyleData {
    pub init_func: IRPointer<Function, 1>,
    pub apply_func: IRPointer<Function, 1>,
    pub struct_ty: IRTypeId,
    pub property_codes: Vec<StyleProperty>,
}

#[derive(Clone)]
pub(crate) struct ResolvedProperty<'a> {
    pub property: StyleProperty,
    pub source: PropertySource<'a>,
    pub hir_type: TypeId,
}

#[derive(Clone)]
pub(crate) enum PropertySource<'a> {
    Inherited(usize),
    Own(&'a StylesDefinition),
}

impl Codegen {
    pub(crate) fn collect_style_properties<'a>(
        &self,
        statements: &'a [HirStyleStatement],
    ) -> Vec<&'a StylesDefinition> {
        let mut props = Vec::new();
        for stmt in statements {
            if let HirStyleStatement::Styles(blocks) = stmt {
                for block in blocks
                    .iter()
                    .filter(|block| matches!(block.kind, HirStyleBlockKind::Default))
                {
                    for def in &block.definitions {
                        props.push(def);
                    }
                }
            }
        }
        props
    }

    fn compute_property_prim_counts(
        &self,
        properties: &[ResolvedProperty],
        hir: &SlynxHir,
    ) -> Vec<usize> {
        properties
            .iter()
            .map(|rp| hir.flatten_type(rp.hir_type).len())
            .collect()
    }

    pub(crate) fn lower_stylesheet(
        &mut self,
        decl: &HirDeclaration,
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let HirDeclarationKind::StyleSheet {
            ref statements,
            ref usages,
            ..
        } = decl.kind
        else {
            unreachable!("lower_stylesheet called on non-stylesheet declaration");
        };

        let own_props = self.collect_style_properties(statements);
        let resolved = self.resolve_style_inheritance(usages, &own_props, hir);

        let style_data = self.styles.get_mut(&decl.id).unwrap();
        style_data.property_codes = resolved.iter().map(|rp| rp.property).collect();

        let struct_ty = self
            .get_mapped_type(&decl.ty)
            .ok_or(CodegenError::IRTypeNotRecognized(decl.ty))?;
        self.populate_style_struct_fields(struct_ty, &resolved, hir, ir)?;

        self.create_style_constructor(decl, struct_ty, usages, &resolved, hir, ir)?;
        self.create_style_apply_function(decl, struct_ty, &resolved, hir, ir)?;

        Ok(())
    }

    pub(crate) fn resolve_style_inheritance<'a>(
        &self,
        usages: &[HirStyleUsage],
        own_props: &[&'a StylesDefinition],
        hir: &SlynxHir,
    ) -> Vec<ResolvedProperty<'a>> {
        let mut resolved: Vec<ResolvedProperty<'a>> = Vec::new();
        let mut seen_codes: HashSet<StyleProperty> = HashSet::new();

        for (usage_idx, usage) in usages.iter().enumerate() {
            let decl = hir.find_declaration(usage.style);
            if let HirDeclarationKind::StyleSheet { ref statements, .. } = decl.kind {
                let parent_props = self.collect_style_properties(statements);
                for def in &parent_props {
                    let name_str = hir.get_name(def.name);
                    let property = StyleProperty::from_name(name_str);
                    if !seen_codes.contains(&property) {
                        seen_codes.insert(property);
                        resolved.push(ResolvedProperty {
                            property,
                            source: PropertySource::Inherited(usage_idx),
                            hir_type: def.expected_type,
                        });
                    }
                }
            }
        }

        for def in own_props {
            let name_str = hir.get_name(def.name);
            let code = StyleProperty::from_name(name_str);
            if let Some(pos) = resolved.iter().position(|rp| rp.property == code) {
                resolved[pos] = ResolvedProperty {
                    property: code,
                    source: PropertySource::Own(def),
                    hir_type: def.expected_type,
                };
            } else {
                seen_codes.insert(code);
                resolved.push(ResolvedProperty {
                    property: code,
                    source: PropertySource::Own(def),
                    hir_type: def.expected_type,
                });
            }
        }

        resolved.sort_by_key(|rp| rp.property);
        resolved
    }

    fn populate_style_struct_fields(
        &mut self,
        struct_ty: IRTypeId,
        properties: &[ResolvedProperty],
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let field_types: Vec<IRTypeId> = properties
            .iter()
            .flat_map(|rp| hir.flatten_type(rp.hir_type))
            .map(|prim_ty| self.get_or_create_ir_type(&prim_ty, hir, ir))
            .collect::<Result<Vec<_>, _>>()?;
        let IRType::Struct(id) = ir.get_type(struct_ty) else {
            unreachable!("Style struct type must be IRType::Struct");
        };
        let struct_obj = ir.get_object_type_mut(id);
        for field_ty in field_types {
            struct_obj.insert_field(field_ty);
        }
        Ok(())
    }

    fn flatten_struct_value(
        &mut self,
        value: Value,
        ty: TypeId,
        hir: &SlynxHir,
        ctx: &mut FunctionContext,
    ) -> Result<Vec<Value>, CodegenError> {
        match &*hir.get_type(&ty) {
            HirType::Int | HirType::Float | HirType::Bool | HirType::Str | HirType::Void => {
                Ok(vec![value])
            }
            HirType::Struct { fields } | HirType::Tuple { fields } => {
                let mut result = Vec::new();
                for (i, field_ty) in fields.iter().enumerate() {
                    let field_val = ctx.get_field(value, i as u16);
                    result.extend(self.flatten_struct_value(field_val, *field_ty, hir, ctx)?);
                }
                Ok(result)
            }
            HirType::Reference { rf, .. } => self.flatten_struct_value(value, *rf, hir, ctx),
            _ => Ok(vec![value]),
        }
    }

    fn create_style_constructor(
        &mut self,
        decl: &HirDeclaration,
        struct_ty: IRTypeId,
        usages: &[HirStyleUsage],
        properties: &[ResolvedProperty],
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let (decl_args, statements) = if let HirDeclarationKind::StyleSheet {
            ref args,
            ref statements,
            ..
        } = decl.kind
        {
            (args, statements)
        } else {
            unreachable!()
        };

        let hir_type_args = if let HirType::Style { args } = &*hir.get_type(&decl.ty) {
            args.clone()
        } else {
            Vec::new()
        };

        let arg_ir_types: Vec<IRTypeId> = hir_type_args
            .iter()
            .map(|a| self.get_or_create_ir_type(a, hir, ir))
            .collect::<Result<Vec<_>, _>>()?;

        let init_func = self.styles[&decl.id].init_func;
        let builder = ir.build_function(init_func);
        let mut ctx = crate::functions::FunctionContext::new(builder);
        let entry = ctx.create_label("entry");
        ctx.switch_to_block(entry).unwrap();
        ctx.set_function_type(arg_ir_types, struct_ty);
        self.map_function_arguments(&mut ctx, decl_args);
        for statement in statements {
            if let HirStyleStatement::Statement(s) = statement {
                self.lower_statement(s, hir, &mut ctx)?;
            }
        }

        let mut parent_structs: Vec<Option<(Value, IRTypeId)>> = vec![None; usages.len()];
        let needed_usages: HashSet<usize> = properties
            .iter()
            .filter_map(|rp| match rp.source {
                PropertySource::Inherited(idx) => Some(idx),
                PropertySource::Own(_) => None,
            })
            .collect();

        for &usage_idx in &needed_usages {
            let usage = &usages[usage_idx];
            let param_values = self.get_usage_args(usage, hir, &mut ctx)?;
            let parent_data = &self.styles[&usage.style];
            let struct_value =
                ctx.call(parent_data.init_func, &param_values, parent_data.struct_ty);
            parent_structs[usage_idx] = Some((struct_value, parent_data.struct_ty));
        }

        let mut field_values = Vec::new();
        for rp in properties {
            let value = match &rp.source {
                PropertySource::Own(def) => self.lower_expression(&def.expr, hir, &mut ctx)?,
                PropertySource::Inherited(usage_idx) => {
                    parent_structs[*usage_idx]
                        .expect("Parent struct should have been computed")
                        .0
                }
            };
            let primitives = self.flatten_struct_value(value, rp.hir_type, hir, &mut ctx)?;
            field_values.extend(primitives);
        }

        let struct_val = ctx.struct_literal(struct_ty, &field_values);
        ctx.ret(struct_val);
        ctx.finish();
        Ok(())
    }

    fn create_style_apply_function(
        &mut self,
        decl: &HirDeclaration,
        struct_ty: IRTypeId,
        properties: &[ResolvedProperty],
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let generic_component_ty = ir.generic_component_type();
        let void_ty = ir.void_type();
        let apply_func = self.styles[&decl.id].apply_func;
        let builder = ir.build_function(apply_func);
        let mut ctx = crate::functions::FunctionContext::new(builder);
        let entry = ctx.create_label("entry");
        ctx.switch_to_block(entry).unwrap();
        let args = ctx
            .set_function_type(vec![generic_component_ty, struct_ty], void_ty)
            .to_vec();

        let comp_value = args[0];
        let struct_value = args[1];

        let prim_counts = self.compute_property_prim_counts(properties, hir);
        let mut field_offset = 0usize;
        for (rp, count) in properties.iter().zip(prim_counts.iter()) {
            let mut sapply_args = vec![comp_value];
            for i in 0..*count {
                let prim_val = ctx.get_field(struct_value, (field_offset + i) as u16);
                sapply_args.push(prim_val);
            }
            ctx.sapply(rp.property, &sapply_args);
            field_offset += count;
        }

        ctx.ret(Value::VOID);
        ctx.finish();
        Ok(())
    }
}
