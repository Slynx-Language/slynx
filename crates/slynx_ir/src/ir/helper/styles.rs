use std::collections::HashSet;

use slynx_hir::{
    DeclarationId,
    model::{
        HirDeclaration, HirDeclarationKind, HirExpression, HirStyleBlockKind, HirStyleStatement,
        HirStyleUsage, StylesDefinition,
    },
};

use crate::{
    Context, IRError, IRPointer, IRTypeId, Instruction, SlynxIR, StyleProperty, Value,
    ir::temp::TempIRData,
};

type StyleValue<'a> = (StyleProperty, &'a HirExpression);

type ResolvedStyle<'a> = (Vec<StyleValue<'a>>, Vec<DeclarationId>);

impl SlynxIR {
    /// Collect style property definitions from a list of style statements.
    fn collect_style_properties<'a>(
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
    /// 3. Create the constructor function that returns the struct with default values.
    /// 4. Create the Apply function that emits @sapply for each property.
    pub(crate) fn lower_stylesheet(
        &mut self,
        decl: &HirDeclaration,
        temp: &mut TempIRData,
    ) -> Result<(), IRError> {
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
        let (merged_props, _parent_usage_decls) =
            self.resolve_style_inheritance(usages, &own_props, temp.hir)?;

        // 3. Populate the struct type fields
        let struct_ty = temp.get_type(decl.ty)?;
        self.populate_style_struct_fields(struct_ty, &merged_props)?;

        // 4. Create the constructor function (builds the struct with default values)
        self.create_style_constructor(decl, struct_ty, &merged_props, temp)?;

        // 5. Create the apply function
        self.create_style_apply_function(decl, struct_ty, &merged_props, temp)?;

        Ok(())
    }

    /// Resolve style inheritance.
    ///
    /// Returns merged properties in STYLES_TABLE order and parent usage declaration IDs.
    fn resolve_style_inheritance<'a>(
        &self,
        usages: &[HirStyleUsage],
        own_props: &[&'a StylesDefinition],
        hir: &'a [HirDeclaration],
    ) -> Result<ResolvedStyle<'a>, IRError> {
        let mut merged: Vec<(StyleProperty, &HirExpression)> = Vec::new();
        let mut seen_codes: HashSet<StyleProperty> = HashSet::new();
        let mut parent_ids: Vec<DeclarationId> = Vec::new();

        for usage in usages {
            parent_ids.push(usage.style);
            let style = &hir[usage.style.as_raw() as usize];
            if let HirDeclarationKind::StyleSheet { ref statements, .. } = style.kind {
                let parent_props = self.collect_style_properties(statements);

                for def in &parent_props {
                    let name_str = self.strings.get_name(def.name);
                    let property = StyleProperty::from_name(name_str);

                    if !seen_codes.contains(&property) {
                        seen_codes.insert(property);
                        merged.push((property, &def.expr));
                    }
                }
            }
        }

        // Own properties override parent properties with the same code
        for def in own_props {
            let name_str = self.strings.get_name(def.name);
            let code = StyleProperty::from_name(name_str);

            if let Some(pos) = seen_codes.iter().position(|c| *c == code) {
                merged[pos] = (code, &def.expr);
            } else {
                seen_codes.insert(code);
                merged.push((code, &def.expr));
            }
        }

        // Sort by STYLES_TABLE code
        merged.sort_by_key(|(code, _)| *code);

        Ok((merged, parent_ids))
    }

    /// Populate the fields of the style struct based on resolved properties.
    fn populate_style_struct_fields(
        &mut self,
        struct_ty: IRTypeId,
        properties: &[(StyleProperty, &HirExpression)],
    ) -> Result<(), IRError> {
        use crate::IRType;

        // Compute all field types first to avoid borrow conflicts
        let field_types: Vec<IRTypeId> = properties
            .iter()
            .map(|(code, _)| code.ir_type(&self.types))
            .collect();

        let struct_id = match self.types.get_type(struct_ty) {
            IRType::Struct(id) => id,
            _ => unreachable!("Style struct type must be IRType::Struct"),
        };
        let struct_obj = self.types.get_object_type_mut(struct_id);

        for field_ty in field_types {
            struct_obj.insert_field(field_ty);
        }

        Ok(())
    }

    /// Create the constructor function for a stylesheet.
    ///
    /// The constructor evaluates the merged property expressions and builds the
    /// struct literal, returning it. Usage sites call this function instead of
    /// inlining the struct literal, keeping defaults in one place.
    fn create_style_constructor(
        &mut self,
        decl: &HirDeclaration,
        struct_ty: IRTypeId,
        properties: &[(StyleProperty, &HirExpression)],
        temp: &mut TempIRData,
    ) -> Result<IRPointer<Context, 1>, IRError> {
        let ctx = temp
            .get_style_init_function(decl.id)
            .expect("Expected style init function to be hoisted");

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
        let prev_func = temp.current_function();
        let prev_label = temp.current_label();
        temp.set_current_function(ctx);
        let entry_label = self.insert_label(ctx, "entry");
        self.get_context_mut(ctx)
            .set_label_ptr(entry_label.with_length());
        temp.set_current_label(entry_label);

        // Evaluate each merged property expression and build struct literal
        let mut field_values = Vec::new();
        for (_, expr) in properties {
            let val = self.get_value_for(expr, temp)?;
            field_values.push(self.get_value(val));
        }

        let fields_ptr = self.insert_values(&field_values);
        let struct_lit = self.insert_instruction(
            temp.current_label(),
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
            temp.current_label(),
            Instruction::ret(struct_value, struct_ty),
            true,
        );

        temp.set_current_function(prev_func);
        temp.set_current_label(prev_label);

        Ok(ctx)
    }

    /// Create the apply function for a stylesheet.
    fn create_style_apply_function(
        &mut self,
        decl: &HirDeclaration,
        struct_ty: IRTypeId,
        properties: &[(StyleProperty, &HirExpression)],

        temp: &mut TempIRData,
    ) -> Result<IRPointer<Context, 1>, IRError> {
        let generic_component_ty = self.types.generic_component_type();
        let void_ty = self.types.void_type();

        // Create function context and set arg/return types
        let ctx = temp
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
        let prev_func = temp.current_function();
        let prev_label = temp.current_label();
        temp.set_current_function(ctx);
        let entry_label = self.insert_label(ctx, "entry");
        self.get_context_mut(ctx)
            .set_label_ptr(entry_label.with_length());
        temp.set_current_label(entry_label);

        // Insert FuncArg values for p0 (component) and p1 (struct)
        let comp_value = self.insert_value(self.generate_func_arg_value(0, temp));
        let struct_value = self.insert_value(self.generate_func_arg_value(1, temp));

        // For each property, emit: getfield + @sapply
        for (field_idx, (code, _)) in properties.iter().enumerate() {
            let field_ty = self.types.get_field_type(struct_ty, field_idx);

            // getfield: extract field from struct
            let getfield_instr = self.insert_instruction(
                temp.current_label(),
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
                temp.current_label(),
                Instruction::sapply(*code, sapply_operands, void_ty),
                true,
            );
        }

        // Emit ret
        let void_value = self.insert_value(self.generate_void_value());
        self.insert_instruction(
            temp.current_label(),
            Instruction::ret(void_value, void_ty),
            true,
        );

        temp.set_current_function(prev_func);
        temp.set_current_label(prev_label);

        Ok(ctx)
    }
}
