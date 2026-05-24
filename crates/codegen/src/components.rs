use slynx_hir::{
    ComponentMemberDeclaration, HirComponentExpression, HirDeclaration,
    HirSpecializedComponentExpression, HirStyleUsage, HirType,
};
use slynx_ir::{Function, IRPointer, IRType, IRTypeId, Value};

use crate::{Codegen, CodegenError};

/// Holds pointers to the two functions generated for a stylesheet, plus
/// the IR type of the style struct:
///   - `init_func`: the constructor (e.g. `__init_Maria`) that builds the style struct
///   - `apply_func`: the apply function (e.g. `Maria`) that reads the struct and emits @sapply
///   - `struct_ty`:  the IR type of the style struct (used to produce constructor call values)
///
/// This data is produced by [`get_style_application`] and is consumed by
/// [`initialize_component`] to emit the style's `@initcall`.
pub struct StyleApplyData {
    init_func: IRPointer<Function, 1>,
    apply_func: IRPointer<Function, 1>,
    struct_ty: IRTypeId,
}

/// Internal bookkeeping for one specialized child's style, used when building
/// the component's UI instruction list in [`initialize_component`].
struct StyleEntry {
    child_idx: usize,
    apply_func: IRPointer<Function, 1>,
    struct_value_ptr: IRPointer<Value, 1>,
}

struct ChildrenValue<'a> {
    values: Vec<Value>,
    expressions: Vec<(usize, &'a HirSpecializedComponentExpression)>,
}

impl Codegen {
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
    ) -> Result<StyleApplyData, CodegenError> {
        let style = self
            .temp
            .get_style(style_usage.style)
            .ok_or(CodegenError::DeclarationNotRecognized(style_usage.style))?;
        Ok(StyleApplyData {
            init_func: style.init_func,
            apply_func: style.apply_func,
            struct_ty: style.strct,
        })
    }

    /// Lowers a [`HirComponentExpression`] into a component IR value and emits
    /// `SetField` (`propset`) instructions for its properties and children in the
    /// **current function Function**.
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
    ) -> Result<(Value, Option<StyleApplyData>), CodegenError> {
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
            HirComponentExpression::Normal { name, .. } => self.temp.get_type(*name)?,
        };

        let label = self.temp.current_label();
        let instruction =
            self.insert_instruction(label, Instruction::component(ty, IRPointer::null()), false);
        let instruction_ptr = self.dereference_instruction_ptr(instruction).with_length();
        let comp_value = Value::new_instruction(instruction_ptr, ty);

        match value {
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Text {
                text,
                style: _,
            }) => {
                let text_value = self.generate_value_for(text)?;
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
                    let (child_value, _) = self.get_component_expression(child)?;
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
                    let prop_value = self.generate_value_for(prop.expr())?;
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
                    let (child_value, _) = self.get_component_expression(child)?;
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
            Some(self.get_style_application(style)?)
        } else {
            None
        };
        Ok((comp_value, style_application))
    }

    /// Returns the IR type for a component expression without emitting any code.
    pub fn get_type_of_component_expression(
        &mut self,
        expr: &HirComponentExpression,
    ) -> Result<IRTypeId, CodegenError> {
        let v = match expr {
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Text {
                ..
            }) => self.types.specialized_text_type(),
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Div {
                ..
            }) => self.types.specialized_div_type(),
            HirComponentExpression::Normal { name, .. } => self.get_ir_type(name)?,
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
    ) -> Result<(), CodegenError> {
        let label = temp.current_label();
        match expr {
            HirSpecializedComponentExpression::Text { text, style: _ } => {
                let text_value = self.generate_value_for(text)?;
                let vals = self.insert_values(&[self.get_value(p0), self.get_value(text_value)]);
                self.insert_instruction(
                    label,
                    Instruction::setfield(0, vals.with_length(), self.types.str_type()),
                    true,
                );
            }
            HirSpecializedComponentExpression::Div { children, style: _ } => {
                for (i, child) in children.iter().enumerate() {
                    let (child_value, _) = self.get_component_expression(child)?;
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
    ) -> Result<IRPointer<Value>, CodegenError> {
        let mut out = Vec::with_capacity(usage.params.len());
        for param in &usage.params {
            let value = self.generate_value_for(param)?;
            let value = self.get_value(value);
            out.push(value);
        }
        Ok(self.insert_values(&out))
    }

    /// Populates the IR component type's field list from the HIR type definition.
    fn populate_component_type_fields(
        &mut self,
        decl: &HirDeclaration,
        component_id: IRComponentId,
    ) -> Result<(), CodegenError> {
        let Some(HirType::Component { props: ty_props }) =
            self.temp.types_module().get_component(&decl.ty)
        else {
            unreachable!("{:?} should map to an Component, but it doesn't", decl);
        };
        for prop_type in ty_props.iter().map(|prop| prop.prop_type()) {
            let ty = self.get_ir_type(prop_type)?;
            let comp_ty = self.types.get_component_type_mut(component_id);
            comp_ty.insert_field(ty);
        }
        Ok(())
    }

    /// Creates IR values for each top-level child of the component and collects
    /// specialized children (Div/Text) for later init / style handling.
    ///
    /// Returns `(ir_values, specialized_children)`.
    fn create_component_child_values<'a>(
        &mut self,
        decl: &HirDeclaration,
        props: &'a [ComponentMemberDeclaration],
        component_id: IRComponentId,
    ) -> Result<ChildrenValue<'a>, CodegenError> {
        let mut ir_values = Vec::new();
        let mut specialized_children: Vec<(usize, &HirSpecializedComponentExpression)> = Vec::new();

        for (idx, prop) in props.iter().enumerate() {
            match prop {
                ComponentMemberDeclaration::Property { value: None, .. } => {}
                ComponentMemberDeclaration::Property {
                    value: Some(value),
                    index,
                    ..
                } => {
                    let value = self.generate_value_for(value)?;
                    temp.get_component_mut(decl.id)
                        .default_properties
                        .push((value, *index as u8));
                }
                ComponentMemberDeclaration::Child(c) => {
                    let ty = self.get_type_of_component_expression(c)?;
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
                        specialized_children.push((idx, spec));
                    }
                }
            }
        }

        Ok(ChildrenValue {
            values: ir_values,
            expressions: specialized_children,
        })
    }

    /// Builds the init function for the first specialized child.
    ///
    /// Sets up a new function (e.g. `PedrinhoInit`) that takes the child value as
    /// `p0` and emits `SetField` instructions for its built-in properties, ending
    /// with a `ret`. Stores the init function pointer in
    /// `self.components[comp_ptr].init_func`.
    ///
    /// Returns the init function pointer.
    fn build_init_function(
        &mut self,
        decl: &HirDeclaration,
        comp_ptr: IRPointer<crate::Component, 1>,
        first_spec: &HirSpecializedComponentExpression,
    ) -> Result<IRPointer<Function, 1>, CodegenError> {
        let init_func = self.temp.get_init_function(decl.id);
        self.components[comp_ptr.ptr()].init_func = Some(init_func);

        let prev_function = self.temp.current_function();
        let prev_label = self.temp.current_label();

        self.temp.set_current_function(init_func);

        let label = self.insert_label(init_func, "entry");
        self.get_context_mut(init_func)
            .set_label_ptr(label.with_length());
        let next = self.get_next_mapeable_instruction_ptr();
        let mut ptr = next.with_length();
        ptr.set_length(ptr.len() - 1);
        self.get_label_mut(label).set_instructions_pointer(ptr);
        temp.set_current_label(label);

        let spec_type = match first_spec {
            HirSpecializedComponentExpression::Text { .. } => self.types.specialized_text_type(),
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
        self.insert_value(self.create_func_arg_value(0));

        self.emit_specialized_component_init(first_spec, arg_ptr)?;

        let void_value = self.insert_value(self.create_void_value());
        self.insert_instruction(
            self.temp.current_label(),
            Instruction::ret(void_value, self.types.void_type()),
            true,
        );

        self.temp.set_current_function(prev_function);
        self.temp.set_current_label(prev_label);

        Ok(init_func)
    }

    /// Emits constructor calls for style structs, the component's `@initcall`, and
    /// style apply `@initcall`s for every specialized child with a style clause.
    ///
    /// This corresponds to Phase 3 of component initialization. The constructor
    /// calls (step 3.1) go into the global instruction pool; the `@initcall`
    /// instructions (steps 3.2 & 3.3) form `self.components[comp_ptr].ui_instruction`.
    fn emit_initcalls(
        &mut self,
        comp_ptr: IRPointer<crate::Component, 1>,
        init_func: IRPointer<Function, 1>,
        specialized_children: &[(usize, &HirSpecializedComponentExpression)],
    ) -> Result<(), CodegenError> {
        let void_ty = self.types.void_type();

        let mut style_entries: Vec<StyleEntry> = Vec::new();
        for (child_idx, spec) in specialized_children {
            let style_usage = match spec {
                HirSpecializedComponentExpression::Text { style, .. } => style.as_ref(),
                HirSpecializedComponentExpression::Div { style, .. } => style.as_ref(),
            };
            if let Some(usage) = style_usage {
                let style_data = self.get_style_application(usage)?;
                let args = self.get_usage_args(usage)?;

                let cons_call = self.insert_instruction(
                    self.temp.current_label(),
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

        let comp_val = self.create_component_child_value(0, comp_ptr);
        let initcall_vals = self.insert_values(&[comp_val]);
        let initcall_ptr = self.insert_instruction(
            self.temp.current_label(),
            Instruction::initcall(init_func, initcall_vals, void_ty),
            false,
        );
        let inst_ptr = self.dereference_instruction_ptr(initcall_ptr);

        for entry in &style_entries {
            let comp_val = self.create_component_child_value(entry.child_idx, comp_ptr);
            let style_vals =
                self.insert_values(&[comp_val, self.get_value(entry.struct_value_ptr)]);
            self.insert_instruction(
                self.temp.current_label(),
                Instruction::initcall(entry.apply_func, style_vals, void_ty),
                false,
            );
        }

        let ui_len = 1 + style_entries.len();
        self.components[comp_ptr.ptr()].ui_instruction = IRPointer::new(inst_ptr.ptr(), ui_len);

        Ok(())
    }

    /// Initializes a component declaration from HIR into IR.
    ///
    /// This is the main entry point for lowering a [`HirDeclarationKind::ComponentDeclaration`].
    /// It delegates to helper methods for each phase:
    ///
    /// 1. [`populate_component_type_fields`] — inserts field types into the IR
    ///    component type.
    /// 2. [`create_component_child_values`] — creates IR values for each child
    ///    and collects specialized children.
    /// 3. [`build_init_function`] — creates an init function for the first
    ///    specialized child (if any).
    /// 4. [`emit_initcalls`] — emits `@initcall` instructions for init and
    ///    style application.
    pub(crate) fn initialize_component(
        &mut self,
        decl: &HirDeclaration,
        props: &[ComponentMemberDeclaration],
    ) -> Result<(), CodegenError> {
        let component_type = self.get_ir_type(&decl.ty)?;
        let IRType::Component(component_id) = self.ir.get(component_type) else {
            unreachable!("Something errored that type of component simply isnt Component on ir");
        };

        self.populate_component_type_fields(decl, component_id)?;

        let comp_ptr = self.temp.get_component(decl.id).ptr;
        let ChildrenValue {
            values: ir_values,
            expressions: specialized_children,
        } = self.create_component_child_values(decl, props, component_id)?;

        self.components[comp_ptr.ptr()].values = self.insert_values(&ir_values);

        if let Some((_, first_spec)) = specialized_children.first() {
            let init_func = self.build_init_function(decl, comp_ptr, first_spec)?;
            self.emit_initcalls(comp_ptr, init_func, &specialized_children)?;
        }

        Ok(())
    }
}
