use std::collections::HashSet;

use slynx_hir::{
    HirDeclaration, HirDeclarationKind, HirStyleBlockKind, HirStyleStatement, HirStyleUsage,
    StylesDefinition,
};
use slynx_ir::{Function, IRPointer, IRType, IRTypeId, StyleProperty};

use crate::{Codegen, CodegenError};

impl Codegen {
    /// Collect style property definitions from a list of style statements.
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

    /// Lower a stylesheet declaration into IR.
    ///
    /// 1. Resolve inheritance from `uses` clauses.
    /// 2. Populate the IR struct type for the style (fields in STYLES_TABLE order).
    /// 3. Store the property-to-field-index mapping for parent lookup.
    /// 4. Create the constructor function that returns the struct with default values.
    /// 5. Create the Apply function that emits @sapply for each property.
    pub(crate) fn lower_stylesheet(&mut self, decl: &HirDeclaration) -> Result<(), CodegenError> {
        let HirDeclarationKind::StyleSheet {
            ref statements,
            ref usages,
            ..
        } = decl.kind
        else {
            unreachable!("lower_stylesheet called on non-stylesheet declaration");
        };

        // 1. Collect own properties from stylesheet body
        let own_props = self.collect_style_properties(statements);

        // 2. Resolve inheritance: merge parent properties with own properties
        let resolved = self.resolve_style_inheritance(usages, &own_props);

        // 3. Populate the struct type fields
        let struct_ty = self.temp.get_type(decl.ty)?;
        self.populate_style_struct_fields(struct_ty, &resolved)?;

        // 4. Create the constructor function (builds the struct with default values)
        self.create_style_constructor(decl, struct_ty, usages, &resolved)?;

        // 5. Create the apply function
        self.create_style_apply_function(decl, struct_ty, &resolved)?;

        Ok(())
    }

    /// Resolve style inheritance.
    ///
    /// Returns merged properties in STYLES_TABLE order, each tagged with its
    /// origin (own expression or inherited from a specific `uses` entry).
    pub(crate) fn resolve_style_inheritance<'a>(
        &self,
        usages: &[HirStyleUsage],
        own_props: &[&'a StylesDefinition],
        hir: &'a [HirDeclaration],
    ) -> Vec<ResolvedProperty<'a>> {
        let mut resolved: Vec<ResolvedProperty<'a>> = Vec::new();
        let mut seen_codes: HashSet<StyleProperty> = HashSet::new();

        for (usage_idx, usage) in usages.iter().enumerate() {
            let style = &hir[usage.style.as_raw() as usize];
            if let HirDeclarationKind::StyleSheet { ref statements, .. } = style.kind {
                let parent_props = self.collect_style_properties(statements);

                for def in &parent_props {
                    let name_str = self.strings.get_name(def.name);
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

        // Own properties override parent properties with the same code
        for def in own_props {
            let name_str = self.strings.get_name(def.name);
            let code = StyleProperty::from_name(name_str);

            if let Some(pos) = resolved.iter().position(|rp| rp.property == code) {
                resolved[pos] = ResolvedProperty {
                    property: code,
                    source: PropertySource::Own(&def.expr),
                };
            } else {
                seen_codes.insert(code);
                resolved.push(ResolvedProperty {
                    property: code,
                    source: PropertySource::Own(&def.expr),
                });
            }
        }

        // Sort by STYLES_TABLE code
        resolved.sort_by_key(|rp| rp.property);

        resolved
    }

    /// Populate the fields of the style struct based on resolved properties.
    fn populate_style_struct_fields(
        &mut self,
        struct_ty: IRTypeId,
        properties: &[ResolvedProperty],
    ) -> Result<(), CodegenError> {
        // Compute all field types first to avoid borrow conflicts
        let field_types: Vec<IRTypeId> = properties
            .iter()
            .map(|rp| rp.property.ir_type(&self.types))
            .collect();

        let struct_id = match self.ir.get_type(struct_ty) {
            IRType::Struct(id) => id,
            _ => unreachable!("Style struct type must be IRType::Struct"),
        };
        let struct_obj = self.ir.get_object_type_mut(struct_id);

        for field_ty in field_types {
            struct_obj.insert_field(field_ty);
        }

        Ok(())
    }

    /// Create the constructor function for a stylesheet.
    ///
    /// For own properties the constructor evaluates the HIR expression directly.
    /// For inherited properties it calls the parent's init function (passing the
    /// evaluated `uses` arguments) and extracts the needed field from the result.
    /// The resulting values are packed into a struct literal and returned.
    fn create_style_constructor(
        &mut self,
        decl: &HirDeclaration,
        struct_ty: IRTypeId,
        usages: &[HirStyleUsage],
        properties: &[ResolvedProperty],
    ) -> Result<IRPointer<Context, 1>, CodegenError> {
        let HirDeclarationKind::StyleSheet { ref args, .. } = decl.kind else {
            unreachable!("Should've received a stylesheet");
        };

        let ctx = self
            .temp
            .get_style_init_function(decl.id)
            .expect("Expected style init function to be hoisted");
        self.temp.set_current_function(ctx);

        let ptr = IRPointer::new(self.values.len(), args.len());
        self.temp.set_function_args(args, ptr);
        for (idx, _) in args.iter().enumerate() {
            self.insert_value(self.create_func_arg_value(idx));
        }

        // Set return type
        {
            use crate::IRType;
            let irty_id = self.get_context(ctx).ty();
            let IRType::Function(func_id) = self.types.get_type(irty_id) else {
                unreachable!();
            };
            let func_ty = self.types.get_function_type_mut(func_id);
            func_ty.set_return_type(struct_ty);
        }

        // Set up entry label
        let prev_func = self.temp.current_function();
        let prev_label = self.temp.current_label();

        let entry_label = self.insert_label(ctx, "entry");
        self.get_context_mut(ctx)
            .set_label_ptr(entry_label.with_length());
        self.temp.set_current_label(entry_label);

        // Phase 1: Call each parent's init function where at least one property is inherited
        let mut parent_structs: Vec<Option<(IRPointer<Value, 1>, IRTypeId)>> =
            vec![None; usages.len()];

        // Collect which usage indices actually contribute properties
        let needed_usages: HashSet<usize> = properties
            .iter()
            .filter_map(|rp| match rp.source {
                PropertySource::Inherited(idx) => Some(idx),
                PropertySource::Own(_) => None,
            })
            .collect();

        for &usage_idx in &needed_usages {
            let usage = &usages[usage_idx];

            // Evaluate usage params (these reference this stylesheet's args, now in scope)
            let args = self.get_usage_args(usage)?;

            // Get parent's init function and struct type
            let parent_init = self
                .temp
                .get_style_init_function(usage.style)
                .expect("Parent style init function should have been hoisted");
            let parent_struct_ty = self
                .temp
                .get_style_struct_type(usage.style)
                .expect("Parent style struct type should exist");

            // Call parent's init function
            let cons_call = self.insert_instruction(
                temp.current_label(),
                Instruction::call(parent_init, parent_struct_ty, args),
                false,
            );
            let cons_ptr = self.dereference_instruction_ptr(cons_call);
            let struct_value = self.insert_value(Value::new_instruction(
                cons_ptr.with_length(),
                parent_struct_ty,
            ));

            parent_structs[usage_idx] = Some((struct_value, parent_struct_ty));
        }

        // Phase 2: Evaluate each property value
        let mut field_values = Vec::new();
        for resolved_prop in properties {
            let value = match &resolved_prop.source {
                PropertySource::Own(expr) => {
                    let val = self.generate_value_for(expr)?;
                    self.get_value(val)
                }
                PropertySource::Inherited(usage_idx) => {
                    let (parent_struct, parent_struct_ty) = parent_structs[*usage_idx]
                        .expect("Parent struct should have been computed");

                    // Find the field index in the parent struct for this property code
                    let parent_style_data = self
                        .temp
                        .get_style(usages[*usage_idx].style)
                        .expect("Parent style data should exist");
                    let field_idx = parent_style_data
                        .property_codes
                        .iter()
                        .position(|c| *c == resolved_prop.property)
                        .expect("Property should exist in parent style struct");

                    let field_ty = self.types.get_field_type(parent_struct_ty, field_idx);

                    let getfield_instr = self.insert_instruction(
                        self.temp.current_label(),
                        Instruction::getfield(field_idx, parent_struct, field_ty),
                        false,
                    );
                    let getfield_ptr = self.dereference_instruction_ptr(getfield_instr);
                    let field_value_ptr = self
                        .insert_value(Value::new_instruction(getfield_ptr.with_length(), field_ty));
                    self.get_value(field_value_ptr)
                }
            };
            field_values.push(value);
        }

        let fields_ptr = self.insert_values(&field_values);
        let struct_lit = self.insert_instruction(
            self.temp.current_label(),
            Instruction::struct_literal(struct_ty, fields_ptr),
            false,
        );
        let struct_lit_ptr = self.dereference_instruction_ptr(struct_lit);
        let struct_value = self.insert_value(Value::new_instruction(
            struct_lit_ptr.with_length(),
            struct_ty,
        ));

        // Return the struct
        self.insert_instruction(
            self.temp.current_label(),
            Instruction::ret(struct_value, struct_ty),
            true,
        );

        self.temp.set_current_function(prev_func);
        self.temp.set_current_label(prev_label);

        Ok(ctx)
    }

    /// Create the apply function for a stylesheet.
    fn create_style_apply_function(
        &mut self,
        decl: &HirDeclaration,
        struct_ty: IRTypeId,
        properties: &[ResolvedProperty],
    ) -> Result<IRPointer<Function, 1>, CodegenError> {
        let generic_component_ty = self.types.generic_component_type();
        let void_ty = self.types.void_type();

        // Create function context and set arg/return types
        let ctx = self
            .temp
            .get_style_apply_function(decl.id)
            .expect("Expected style apply function to be hoisted");
        {
            use crate::IRType;
            let irty_id = self.get_context(ctx).ty();
            let IRType::Function(func_id) = self.types.get_type(irty_id) else {
                unreachable!();
            };
            let func_ty = self.types.get_function_type_mut(func_id);
            func_ty.insert_arg_types(&[generic_component_ty, struct_ty]);
            func_ty.set_return_type(void_ty);
        }

        // Set up entry label, saving and restoring prior temp state
        let prev_func = self.temp.current_function();
        let prev_label = self.temp.current_label();
        self.temp.set_current_function(ctx);
        let entry_label = self.insert_label(ctx, "entry");
        self.get_context_mut(ctx)
            .set_label_ptr(entry_label.with_length());
        self.temp.set_current_label(entry_label);
        // Insert FuncArg values for p0 (component) and p1 (struct)
        let comp_value = self.insert_value(self.create_func_arg_value(0, self.temp));
        let struct_value = self.insert_value(self.create_func_arg_value(1, self.temp));

        // For each property, emit: getfield + @sapply
        for (field_idx, rp) in properties.iter().enumerate() {
            let field_ty = self.types.get_field_type(struct_ty, field_idx);

            // getfield: extract field from struct
            let getfield_instr = self.insert_instruction(
                self.temp.current_label(),
                Instruction::getfield(field_idx, struct_value, field_ty),
                false,
            );
            let getfield_ptr = self.dereference_instruction_ptr(getfield_instr);
            let field_value =
                self.insert_value(Value::new_instruction(getfield_ptr.with_length(), field_ty));

            // @sapply: apply property to component
            let operands = {
                let comp = self.get_value(comp_value);
                let field = self.get_value(field_value);
                self.insert_values(&[comp, field])
            };
            let sapply_operands = operands.with_runtime_length(2);

            self.insert_instruction(
                self.temp.current_label(),
                Instruction::sapply(rp.property, sapply_operands, void_ty),
                true,
            );
        }

        // Emit ret
        let void_value = self.insert_value(self.create_void_value());
        self.insert_instruction(
            self.temp.current_label(),
            Instruction::ret(void_value, void_ty),
            true,
        );

        self.temp.set_current_function(prev_func);
        self.temp.set_current_label(prev_label);

        Ok(ctx)
    }
}
