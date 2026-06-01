use std::collections::HashSet;

use slynx_hir::{
    HirDeclaration, HirDeclarationKind, HirStyleBlockKind, HirStyleStatement, HirStyleUsage,
    SlynxHir, StylesDefinition,
};
use slynx_ir::{Function, IRPointer, IRType, IRTypeId, SlynxIR, StyleProperty, Value};

use crate::{Codegen, CodegenError};

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

        let struct_ty = self
            .get_mapped_type(&decl.ty)
            .ok_or(CodegenError::IRTypeNotRecognized(decl.ty))?;
        self.populate_style_struct_fields(struct_ty, &resolved, ir)?;

        let style_data = self.styles.get_mut(&decl.id).unwrap();
        style_data.property_codes = resolved.iter().map(|rp| rp.property).collect();

        self.create_style_constructor(decl, struct_ty, usages, &resolved, hir, ir)?;
        self.create_style_apply_function(decl, struct_ty, &resolved, ir)?;

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
            let decl = &hir.declarations[usage.style.as_raw() as usize];
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
                };
            } else {
                seen_codes.insert(code);
                resolved.push(ResolvedProperty {
                    property: code,
                    source: PropertySource::Own(def),
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
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let field_types: Vec<IRTypeId> = properties
            .iter()
            .map(|rp| rp.property.ir_type(ir))
            .collect();

        let IRType::Struct(id) = ir.get_type(struct_ty) else {
            unreachable!("Style struct type must be IRType::Struct");
        };
        let struct_obj = ir.get_object_type_mut(id);
        for field_ty in field_types {
            struct_obj.insert_field(field_ty);
        }
        Ok(())
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

        let hir_type_args = if let slynx_hir::HirType::Style { args } = hir.get_type(&decl.ty) {
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
        for resolved_prop in properties {
            let value = match &resolved_prop.source {
                PropertySource::Own(def) => self.lower_expression(&def.expr, hir, &mut ctx)?,
                PropertySource::Inherited(usage_idx) => {
                    let (struct_val, _) = parent_structs[*usage_idx]
                        .expect("Parent struct should have been computed");
                    let parent_data = &self.styles[&usages[*usage_idx].style];
                    let field_idx = parent_data
                        .property_codes
                        .iter()
                        .position(|c| *c == resolved_prop.property)
                        .expect("Property should exist in parent style struct");

                    ctx.get_field(struct_val, field_idx as u16)
                }
            };
            field_values.push(value);
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

        for (field_idx, rp) in properties.iter().enumerate() {
            let field_value = ctx.get_field(struct_value, field_idx as u16);
            ctx.sapply(rp.property, &[comp_value, field_value]);
        }

        ctx.ret(Value::VOID);
        ctx.finish();
        Ok(())
    }
}
