use slynx_hir::model::{
    ComponentMemberDeclaration, HirComponentExpression, HirDeclaration,
    HirSpecializedComponentExpression, HirStyleUsage, HirType,
};

use crate::{
    Context, IRError, IRPointer, IRType, IRTypeId, Instruction, SlynxIR, Value,
    ir::temp::TempIRData,
};

/// Holds pointers to the two functions generated for a stylesheet, plus
/// the IR type of the style struct:
///   - `init_func`: the constructor (e.g. `__init_Maria`) that builds the style struct
///   - `apply_func`: the apply function (e.g. `Maria`) that reads the struct and emits @sapply
///   - `struct_ty`:  the IR type of the style struct (used to produce constructor call values)
///
/// This data is produced by [`get_style_application`] and is consumed by
/// [`initialize_component`] to emit the style's `@initcall`.
pub struct StyleApplyData {
    init_func: IRPointer<Context, 1>,
    apply_func: IRPointer<Context, 1>,
    struct_ty: IRTypeId,
}

/// Internal bookkeeping for one specialized child's style, used when building
/// the component's UI instruction list in [`initialize_component`].
struct StyleEntry {
    child_idx: usize,
    apply_func: IRPointer<Context, 1>,
    struct_value_ptr: IRPointer<Value, 1>,
}

impl SlynxIR {
    /// Resolves a [`HirStyleUsage`] into the IR function pointers and struct type
    /// for a stylesheet.
    ///
    /// The returned [`StyleApplyData`] contains:
    /// - `init_func`  – call this to produce the style struct value
    /// - `apply_func` – call this with (component, struct) to apply every property
    /// - `struct_ty`  – the IR type of the style struct
    fn get_style_application(
        &mut self,
        style_usage: &HirStyleUsage,
        temp: &mut TempIRData,
    ) -> Result<StyleApplyData, IRError> {
        let style = temp
            .get_style(style_usage.style)
            .ok_or(IRError::DeclarationNotRecognized(style_usage.style))?;
        Ok(StyleApplyData {
            init_func: style.init_func,
            apply_func: style.apply_func,
            struct_ty: style.strct,
        })
    }

    /// Lowers a [`HirComponentExpression`] into a component IR value and emits
    /// `SetField` (`propset`) instructions for its properties and children in the
    /// **current function context**.
    ///
    /// The returned `(Value, Option<StyleApplyData>)` pair carries the component
    /// value and, if the expression is a specialized (Div/Text) node with a
    /// `style:` clause, the style's application data. Callers that build the
    /// component's top-level `@initcall` list (see [`initialize_component`]) use
    /// this style data to emit a second `@initcall` for the style apply function.
    ///
    /// ## When this is called
    /// - **Inside a component declaration** (via [`initialize_component`]) for each
    ///   top-level child – the style data is handled later.
    /// - **Inside a function body** (via [`get_value_for`]) – the style data is
    ///   discarded because function-level component expressions are inlined into
    ///   the parent component's init.
    pub(crate) fn get_component_expression(
        &mut self,
        value: &HirComponentExpression,
        temp: &mut TempIRData,
    ) -> Result<(Value, Option<StyleApplyData>), IRError> {
        let mut style_usage: Option<&HirStyleUsage> = None;
        let ty = match value {
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Text {
                style,
                ..
            }) => {
                style_usage = style.as_ref();
                self.types.specialized_text_type()
            }
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Div {
                style,
                ..
            }) => {
                style_usage = style.as_ref();
                self.types.specialized_div_type()
            }
            HirComponentExpression::Normal { name, .. } => temp.get_type(*name)?,
        };

        let label = temp.current_label();
        let instruction =
            self.insert_instruction(label, Instruction::component(ty, IRPointer::null()), false);
        let instruction_ptr = self.dereference_instruction_ptr(instruction).with_length();
        let comp_value = Value::new_instruction(instruction_ptr, ty);

        match value {
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Text {
                text,
                style: _,
            }) => {
                let text_value = self.get_value_for(text, temp)?;
                let vals = self.insert_values(&[comp_value.clone(), self.get_value(text_value)]);
                self.insert_instruction(
                    label,
                    Instruction::setfield(0, vals.with_length(), self.types.str_type()),
                    true,
                );
            }
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Div {
                children,
                style: _,
            }) => {
                for (i, child) in children.iter().enumerate() {
                    let (child_value, _) = self.get_component_expression(child, temp)?;
                    let vals = self.insert_values(&[comp_value.clone(), child_value]);
                    self.insert_instruction(
                        label,
                        Instruction::setfield(
                            i,
                            vals.with_length(),
                            self.types.generic_component_type(),
                        ),
                        true,
                    );
                }
            }
            HirComponentExpression::Normal {
                properties,
                children,
                ..
            } => {
                for prop in properties {
                    let prop_value = self.get_value_for(prop.expr(), temp)?;
                    let vals =
                        self.insert_values(&[comp_value.clone(), self.get_value(prop_value)]);
                    self.insert_instruction(
                        label,
                        Instruction::setfield(
                            prop.index(),
                            vals.with_length(),
                            self.types.generic_component_type(),
                        ),
                        true,
                    );
                }
                for (i, child) in children.iter().enumerate() {
                    let (child_value, _) = self.get_component_expression(child, temp)?;
                    let vals = self.insert_values(&[comp_value.clone(), child_value]);
                    self.insert_instruction(
                        label,
                        Instruction::setfield(
                            i,
                            vals.with_length(),
                            self.types.generic_component_type(),
                        ),
                        true,
                    );
                }
            }
        }

        let style_application = if let Some(style) = style_usage {
            Some(self.get_style_application(style, temp)?)
        } else {
            None
        };
        Ok((comp_value, style_application))
    }

    /// Returns the IR type for a component expression without emitting any code.
    pub fn get_type_of_component_expression(
        &mut self,
        expr: &HirComponentExpression,
        temp: &TempIRData,
    ) -> Result<IRTypeId, IRError> {
        let v = match expr {
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Text {
                ..
            }) => self.types.specialized_text_type(),
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Div {
                ..
            }) => self.types.specialized_div_type(),
            HirComponentExpression::Normal { name, .. } => self.get_ir_type(name, temp)?,
        };
        Ok(v)
    }

    /// Emits `SetField` (`propset`) instructions inside the component's **init
    /// function** for a specialized (Div/Text) child's built-in properties (text
    /// content, child components).
    ///
    /// Style application is NOT emitted here — it is handled at a higher level
    /// in [`initialize_component`] via a separate `@initcall` so that the apply
    /// function can be called separately from the init function.
    fn emit_specialized_component_init(
        &mut self,
        expr: &HirSpecializedComponentExpression,
        p0: IRPointer<Value, 1>,
        temp: &mut TempIRData,
    ) -> Result<(), IRError> {
        let label = temp.current_label();
        match expr {
            HirSpecializedComponentExpression::Text { text, style: _ } => {
                let text_value = self.get_value_for(text, temp)?;
                let vals = self.insert_values(&[self.get_value(p0), self.get_value(text_value)]);
                self.insert_instruction(
                    label,
                    Instruction::setfield(0, vals.with_length(), self.types.str_type()),
                    true,
                );
            }
            HirSpecializedComponentExpression::Div { children, style: _ } => {
                for (i, child) in children.iter().enumerate() {
                    let (child_value, _) = self.get_component_expression(child, temp)?;
                    let vals = self.insert_values(&[self.get_value(p0), child_value]);
                    self.insert_instruction(
                        label,
                        Instruction::setfield(
                            i,
                            vals.with_length(),
                            self.types.generic_component_type(),
                        ),
                        true,
                    );
                }
            }
        }
        Ok(())
    }

    ///Retrieves the arguments required for the given `usage` style
    pub fn get_usage_args(
        &mut self,
        usage: &HirStyleUsage,
        temp: &mut TempIRData,
    ) -> Result<IRPointer<Value>, IRError> {
        let mut out = Vec::with_capacity(usage.params.len());
        for param in &usage.params {
            let value = self.get_value_for(param, temp)?;
            let value = self.get_value(value);
            out.push(value);
        }
        Ok(self.insert_values(&out))
    }

    /// Initializes a component declaration from HIR into IR.
    ///
    /// This is the main entry point for lowering a [`HirDeclarationKind::ComponentDeclaration`].
    /// It performs three phases:
    ///
    /// **Phase 1 – Top-level values:** Creates IR component instructions for each
    /// child and stores them in the component's `values` list. Collects all
    /// specialized children (Div/Text) for later init and style handling.
    ///
    /// **Phase 2 – Init function:** If the first child is specialized, creates an
    /// init function (e.g. `PedrinhoInit`) that emits `SetField` instructions for
    /// that child's built-in properties. This function is stored in the component's
    /// `init_func` field. Only the **first** specialized child gets an init function.
    ///
    /// **Phase 3 – UI instructions (initcalls):** Emits `@initcall` instructions
    /// into the component's `ui_instruction` list:
    ///   1. Emits constructor calls (unmapped) for every specialized child that has
    ///      a `style:` clause. These produce the style struct values and are stored
    ///      in the global instruction pool (NOT in `ui_instruction`).
    ///   2. Always emits `@initcall ComponentInit, #t0` (calls the init function
    ///      from Phase 2 to set up the first child).
    ///   3. For **every** specialized child (including the first) that has a
    ///      `style:` clause, emits `@initcall ApplyStyle, #t_idx, __init_StyleName()`
    ///      — a two-operand initcall referencing the constructor result from step 1.
    pub(crate) fn initialize_component(
        &mut self,
        decl: &HirDeclaration,
        props: &[ComponentMemberDeclaration],
        temp: &mut TempIRData,
    ) -> Result<(), IRError> {
        let component_type = self.get_ir_type(&decl.ty, temp)?;
        let IRType::Component(component_id) = self.types.get_type(component_type) else {
            unreachable!("Something errored that type of component simply isnt Component on ir");
        };
        {
            let Some(HirType::Component { props: ty_props }) =
                temp.types_module().get_component(&decl.ty)
            else {
                unreachable!("{:?} should map to an Component, but it doesn't", decl);
            };
            for prop_type in ty_props.iter().map(|prop| prop.prop_type()) {
                let ty = self.get_ir_type(prop_type, temp)?;
                let comp_ty = self.types.get_component_type_mut(component_id);
                comp_ty.insert_field(ty);
            }
        }

        let comp_ptr = temp.get_component(decl.id).ptr;
        let mut ir_values = Vec::new();
        let mut specialized_children: Vec<(usize, &HirSpecializedComponentExpression)> = Vec::new();

        // Phase 1: Create top-level child values in the main context.
        // For top-level children, only create the component VALUE without
        // emitting nested SetField instructions. The init function handles setup.
        let mut child_counter = 0;
        for (_idx, prop) in props.iter().enumerate() {
            match prop {
                ComponentMemberDeclaration::Property { value: None, .. } => {}
                ComponentMemberDeclaration::Property {
                    value: Some(value),
                    index,
                    ..
                } => {
                    let value = self.get_value_for(value, temp)?;
                    temp.get_component_mut(decl.id)
                        .default_properties
                        .push((value, *index as u8));
                }
                ComponentMemberDeclaration::Child(c) => {
                    let ty = self.get_type_of_component_expression(c, temp)?;
                    {
                        let comp_ty = self.types.get_component_type_mut(component_id);
                        comp_ty.insert_child(ty);
                    }
                    let inst = self.insert_instruction(
                        temp.current_label(),
                        Instruction::component(ty, IRPointer::null()),
                        false,
                    );
                    let inst_ptr = self.dereference_instruction_ptr(inst).with_length();
                    let value = Value::new_instruction(inst_ptr, ty);
                    ir_values.push(value);

                    if let HirComponentExpression::Specialized(spec) = c {
                        specialized_children.push((child_counter, spec));
                    }
                    child_counter += 1;
                }
            }
        }

        self.components[comp_ptr.ptr()].values = self.insert_values(&ir_values);

        // Phase 2: Set up init function if the first child is specialized.
        if let Some((_, first_spec)) = specialized_children.first() {
            let init_func = temp.get_init_function(decl.id);
            self.components[comp_ptr.ptr()].init_func = Some(init_func);

            let prev_function = temp.current_function();
            let prev_label = temp.current_label();

            temp.set_current_function(init_func);

            let label = self.insert_label(init_func, "entry");
            self.get_context_mut(init_func)
                .set_label_ptr(label.with_length());
            let next = self.get_next_mapeable_instruction_ptr();
            let mut ptr = next.with_length();
            ptr.set_length(ptr.len() - 1);
            self.get_label_mut(label).set_instructions_pointer(ptr);
            temp.set_current_label(label);

            // Set p0 (FuncArg(0)) as the first specialized child's type.
            let spec_type = match first_spec {
                HirSpecializedComponentExpression::Text { .. } => {
                    self.types.specialized_text_type()
                }
                HirSpecializedComponentExpression::Div { .. } => self.types.specialized_div_type(),
            };
            let arg_ptr = IRPointer::new(self.values.len(), 1);
            {
                let ctx_ty = self.get_context(init_func).ty();
                let void_ty = self.types.void_type();
                let IRType::Function(func_id) = self.types.get_type(ctx_ty) else {
                    unreachable!()
                };
                let func = self.types.get_function_type_mut(func_id);
                func.insert_arg_types(&[spec_type]);
                func.set_return_type(void_ty);
            }
            self.insert_value(self.generate_func_arg_value(0, temp));

            self.emit_specialized_component_init(first_spec, arg_ptr, temp)?;

            // Emit ret (mapped so it shows in the init function's label).
            let void_value = self.insert_value(self.generate_void_value());
            self.insert_instruction(
                temp.current_label(),
                Instruction::ret(void_value, self.types.void_type()),
                true,
            );

            // Restore main context.
            temp.set_current_function(prev_function);
            temp.set_current_label(prev_label);

            // Register component properties as symbolic ComponentProperty values
            // so that style usage params (e.g. Fg(color)) can reference them.
            // Properties are reactive fields, not plain variables, so we emit a
            // runtime handle (#pN) instead of an evaluated constant.
            for prop in props {
                if let ComponentMemberDeclaration::Property {
                    variable_id, index, ..
                } = prop
                {
                    let prop_value = self.generate_component_property_value(*index, comp_ptr);
                    let prop_ptr = self.insert_value(prop_value);
                    temp.add_variable(*variable_id, prop_ptr);
                }
            }

            // Phase 3: Emit @initcall instructions into the component's ui_instruction list.
            let void_ty = self.types.void_type();

            // Step 3.1: Emit constructor calls (unmapped) for every specialized
            // child that has a style. These go into the global instruction pool
            // but NOT into ui_instruction — each style initcall references its
            // constructor result via Value::Instruction.
            let mut style_entries: Vec<StyleEntry> = Vec::new();
            for (child_idx, spec) in &specialized_children {
                let style_usage = match spec {
                    HirSpecializedComponentExpression::Text { style, .. } => style.as_ref(),
                    HirSpecializedComponentExpression::Div { style, .. } => style.as_ref(),
                };
                if let Some(usage) = style_usage {
                    let style_data = self.get_style_application(usage, temp)?;
                    let args = self.get_usage_args(usage, temp)?;

                    let cons_call = self.insert_instruction(
                        temp.current_label(),
                        Instruction::call(style_data.init_func, style_data.struct_ty, args),
                        false,
                    );
                    let cons_ptr = self.dereference_instruction_ptr(cons_call);
                    let struct_value = self.insert_value(Value::new_instruction(
                        cons_ptr.with_length(),
                        style_data.struct_ty,
                    ));

                    style_entries.push(StyleEntry {
                        child_idx: *child_idx,
                        apply_func: style_data.apply_func,
                        struct_value_ptr: struct_value,
                    });
                }
            }

            // Step 3.2: Emit the component's own initcall (always present for
            // first specialized child):
            //   @initcall ComponentInit, #t0;
            let comp_val = self.generate_component_child_value(0, comp_ptr);
            let initcall_vals = self.insert_values(&[comp_val]);
            let initcall_ptr = self.insert_instruction(
                temp.current_label(),
                Instruction::initcall(init_func, initcall_vals, void_ty),
                false,
            );
            let inst_ptr = self.dereference_instruction_ptr(initcall_ptr);

            // Step 3.3: Emit a style initcall for every specialized child that
            // has a style:
            //   @initcall ApplyStyle, #t_idx, __init_StyleName();
            for entry in &style_entries {
                let comp_val = self.generate_component_child_value(entry.child_idx, comp_ptr);
                let style_vals =
                    self.insert_values(&[comp_val, self.get_value(entry.struct_value_ptr)]);
                self.insert_instruction(
                    temp.current_label(),
                    Instruction::initcall(entry.apply_func, style_vals, void_ty),
                    false,
                );
            }

            // ui_instruction spans the component initcall + all style initcalls.
            // The constructor calls (Step 3.1) are before them in the instruction
            // stream and are NOT included.
            let ui_len = 1 + style_entries.len();
            self.components[comp_ptr.ptr()].ui_instruction = IRPointer::new(inst_ptr.ptr(), ui_len);
        }

        Ok(())
    }
}
