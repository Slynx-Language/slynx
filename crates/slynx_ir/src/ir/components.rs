use slynx_hir::{
    TypeId,
    model::{
        ComponentMemberDeclaration, HirComponentExpression, HirDeclaration,
        HirSpecializedComponentExpression, HirType,
    },
};

use crate::{
    IRError, IRPointer, IRSpecializedComponent, IRType, Instruction, SlynxIR, Value,
    ir::temp::TempIRData,
};

impl SlynxIR {
    ///Gets a Specialized component on this ir by its provided `ptr`
    pub fn get_specialized(
        &self,
        ptr: IRPointer<IRSpecializedComponent, 1>,
    ) -> &IRSpecializedComponent {
        &self.specialized[ptr.ptr()]
    }
    ///Inserts the given `specialized` component and returns its pointer(or, id)
    pub(crate) fn insert_specialized(
        &mut self,
        specialized: IRSpecializedComponent,
    ) -> IRPointer<IRSpecializedComponent, 1> {
        let ptr = IRPointer::new(self.specialized.len(), 1);
        self.specialized.push(specialized);
        ptr
    }

    ///Gets a value to represent a component whose name(in this case, its type) is the given `name` and the values of it are `values`.
    pub(crate) fn get_component_expression(
        &mut self,
        value: &HirComponentExpression,
        temp: &mut TempIRData,
    ) -> Result<Value, IRError> {
        let mut vals = Vec::new();
        let ty = match value {
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Text {
                text,
            }) => {
                let value = self.get_value_for(&*text, temp)?;
                let value = self.get_value(value);
                vals.push(value);
                self.types.specialized_text_type()
            }
            HirComponentExpression::Specialized(HirSpecializedComponentExpression::Div {
                children,
            }) => {
                for child in children {
                    let value = self.get_component_expression(&child, temp)?;
                    vals.push(value);
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
                    vals.push(self.get_component_expression(child, temp)?);
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

        Ok(Value::Instruction(instruction))
    }

    ///Initializes a component, with both its type, and expressions for children
    pub(crate) fn initialize_component(
        &mut self,
        decl: &HirDeclaration,
        props: &[ComponentMemberDeclaration],
        temp: &mut TempIRData,
    ) -> Result<(), IRError> {
        {
            //initializes the type
            let component_type = self.get_ir_type(&decl.ty, temp)?;
            let IRType::Component(cid) = self.types.get_type(component_type) else {
                unreachable!(
                    "Something errored that type of component simply isnt Component on ir"
                );
            };

            let Some(HirType::Component { props: ty_props }) =
                temp.types_module().get_component(&decl.ty)
            else {
                unreachable!("{:?} should map to an Component, but it doesn't", decl);
            };

            for prop_type in ty_props.iter().map(|prop| prop.prop_type()) {
                let ty = self.get_ir_type(prop_type, temp)?;
                let comp_ty = self.types.get_component_type_mut(cid);
                comp_ty.insert_field(ty);
            }
        }

        let mut ir_values = Vec::new();
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
                    let value = self.get_component_expression(c, temp)?;
                    ir_values.push(value);
                }
            }
        }

        // Armazena os children (ir_values) no componente
        let comp_ptr = temp.get_component(decl.id).ptr;
        self.components[comp_ptr.ptr()].values = self.insert_values(&ir_values);

        Ok(())
    }
}
