use slynx_hir::model::{
    ComponentMemberDeclaration, HirComponentExpression, HirDeclaration,
    HirSpecializedComponentExpression, HirStyleUsage, HirType,
};

use crate::{
    Context, IRError, IRPointer, IRSpecializedComponent, IRType, IRTypeId, Instruction, SlynxIR,
    Value, ir::temp::TempIRData,
};

pub struct StyleApplyData {
    ///Function used to initialize style struct
    init_func: IRPointer<Context, 1>,
    ///Function used to apply the style struct into some Specialized component
    apply_func: IRPointer<Context, 1>,
}

impl SlynxIR {
    ///Gets a Specialized component on this ir by its provided `ptr`
    pub fn get_specialized(
        &self,
        ptr: IRPointer<IRSpecializedComponent, 1>,
    ) -> &IRSpecializedComponent {
        &self.specialized[ptr.ptr()]
    }
    ///Inserts the given `specialized` component and returns its pointer(or, id)
    #[allow(dead_code)]
    pub(crate) fn insert_specialized(
        &mut self,
        specialized: IRSpecializedComponent,
    ) -> IRPointer<IRSpecializedComponent, 1> {
        let ptr = IRPointer::new(self.specialized.len(), 1);
        self.specialized.push(specialized);
        ptr
    }

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

    ///Gets a value to represent a component whose name(in this case, its type) is the given `name` and the values of it are `values`.
    ///Returns the component value, and the ID of the style to be applied, if some. The ID is mainly used on initialize_component, to generate the
    ///@initcall operations
    pub(crate) fn get_component_expression(
        &mut self,
        value: &HirComponentExpression,
        temp: &mut TempIRData,
    ) -> Result<(Value, Option<StyleApplyData>), IRError> {
        let mut style_id = None;
        let mut vals = Vec::new();
        let ty = match value {
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Text {
                text,
                style,
            }) => {
                let value = self.get_value_for(&*text, temp)?;
                let value = self.get_value(value);
                vals.push(value);
                style_id = style.as_ref();
                self.types.specialized_text_type()
            }
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Div {
                children,
                style,
            }) => {
                for child in children {
                    let value = self.get_component_expression(&child, temp)?.0;
                    vals.push(value);
                    style_id = style.as_ref();
                }
                self.types.specialized_div_type()
            }
            HirComponentExpression::Normal {
                name,
                properties,
                children,
                ..
            } => {
                for prop in properties {
                    let value = self.get_value_for(prop.expr(), temp)?;
                    let value = self.get_value(value);
                    vals.push(value);
                }
                for child in children {
                    vals.push(self.get_component_expression(child, temp)?.0);
                }
                temp.get_type(*name)?
            }
        };

        let vals = self.insert_values(&vals);

        let instruction = self.insert_instruction(
            temp.current_label(),
            Instruction::component(ty, vals),
            false,
        );
        let instruction = self.dereference_instruction_ptr(instruction).with_length();
        let style_application = if let Some(style) = style_id {
            Some(self.get_style_application(style, temp)?)
        } else {
            None
        };
        Ok((Value::Instruction(instruction), style_application))
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

    ///Initializes a component, with both its type, and expressions for children
    pub(crate) fn initialize_component(
        &mut self,
        decl: &HirDeclaration,
        props: &[ComponentMemberDeclaration],
        temp: &mut TempIRData,
    ) -> Result<(), IRError> {
        //initializes the type
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
        let mut init_styles = Vec::new();
        let mut ir_values = Vec::new();
        struct ChildToApply {}
        for prop in props {
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
                    {
                        let ty = self.get_type_of_component_expression(c, temp)?;
                        let comp_ty = self.types.get_component_type_mut(component_id);
                        comp_ty.insert_field(ty);
                    }
                    let (value, decl_id) = self.get_component_expression(c, temp)?;
                    if let Some(decl_id) = decl_id {
                        init_styles.push((decl_id, ir_values.len()));
                    }
                    ir_values.push(value);
                }
            }
        }

        // Armazena os children (ir_values) no componente
        let comp_ptr = temp.get_component(decl.id).ptr;
        self.components[comp_ptr.ptr()].values = self.insert_values(&ir_values);
        let call_instructions = init_styles
            .iter()
            .map(|(StyleApplyData { init_func, .. }, _)| {
                let ret_ty = self.return_type_of_context(*init_func);
                let empty_args = self.insert_values(&[]);

                self.insert_instruction(
                    temp.current_label(),
                    Instruction::call(*init_func, ret_ty, empty_args),
                    false,
                )
                .right()
                .unwrap()
            })
            .collect::<Vec<_>>();

        let ptr = IRPointer::new(self.instructions.len(), init_styles.len());
        let void_ty = self.types.void_type();
        for ((apply_data, child_index), call_ptr) in init_styles.iter().zip(call_instructions) {
            let struct_value = self.insert_value(Value::Instruction(call_ptr.with_length()));
            let struct_params = self.get_value(struct_value);

            let values = self.insert_values(&[Value::ComponentChild(*child_index), struct_params]);
            self.insert_instruction(
                temp.current_label(),
                Instruction::initcall(apply_data.apply_func, values, void_ty),
                false,
            );
        }

        self.components[comp_ptr.ptr()].ui_instruction = ptr;
        Ok(())
    }
}
