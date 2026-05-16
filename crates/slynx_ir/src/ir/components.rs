use slynx_hir::model::{
    ComponentMemberDeclaration, HirComponentExpression, HirDeclaration,
    HirSpecializedComponentExpression, HirStyleUsage, HirType,
};

use crate::{
    Context, IRError, IRPointer, IRType, IRTypeId, Instruction, SlynxIR, Value,
    ir::temp::TempIRData,
};

pub struct StyleApplyData {
    init_func: IRPointer<Context, 1>,
    apply_func: IRPointer<Context, 1>,
}

impl SlynxIR {
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
        })
    }

    ///Creates a component value and emits SetField (propset) instructions for
    ///its properties and children in the current context.
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
                let text_value = self.get_value_for(&*text, temp)?;
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
            HirComponentExpression::Normal { name, .. } => self.get_ir_type(&name, temp)?,
        };
        Ok(v)
    }

    ///Emits SetField (propset) instructions for a specialized component's
    ///properties and children, targeting `p0` in the init function context.
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

    ///Initializes a component: creates child values in the main context,
    ///sets up the init function with propset instructions, and adds @initcall.
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
        let mut first_specialized: Option<(usize, &HirSpecializedComponentExpression)> = None;

        // Phase 1: Create top-level child values in the main context
        // For top-level children, only create the component VALUE without
        // emitting nested SetField instructions. The init function handles setup.
        for (idx, prop) in props.iter().enumerate() {
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

                    if idx == 0 {
                        if let HirComponentExpression::Specialized(spec) = c {
                            first_specialized = Some((idx, spec));
                        }
                    }
                }
            }
        }

        self.components[comp_ptr.ptr()].values = self.insert_values(&ir_values);

        // Phase 2: Set up init function if the first child is specialized
        if let Some((_, spec)) = first_specialized {
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

            // Set p0 (FuncArg(0)) as the first specialized child's type
            let spec_type = match spec {
                HirSpecializedComponentExpression::Text { .. } => {
                    self.types.specialized_text_type()
                }
                HirSpecializedComponentExpression::Div { .. } => self.types.specialized_div_type(),
            };
            let arg_ptr = IRPointer::new(self.values.len(), 1);
            self.insert_value(self.generate_func_arg_value(0, temp));
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

            self.emit_specialized_component_init(spec, arg_ptr, temp)?;

            // Emit ret (mapped so it shows in the init function's label)
            let void_value = self.insert_value(self.generate_void_value());
            self.insert_instruction(
                temp.current_label(),
                Instruction::ret(void_value, self.types.void_type()),
                true,
            );

            // Restore main context
            temp.set_current_function(prev_function);
            temp.set_current_label(prev_label);

            // Phase 3: Add @initcall to component ui_instructions
            let void_ty = self.types.void_type();
            let initcall_vals =
                self.insert_values(&[self.generate_component_child_value(0, comp_ptr)]);
            let initcall_ptr = self.insert_instruction(
                temp.current_label(),
                Instruction::initcall(init_func, initcall_vals, void_ty),
                false,
            );
            let inst_ptr = self.dereference_instruction_ptr(initcall_ptr);
            let ui_ptr = IRPointer::new(inst_ptr.ptr(), 1);
            self.components[comp_ptr.ptr()].ui_instruction = ui_ptr;
        }

        Ok(())
    }
}
