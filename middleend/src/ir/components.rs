use frontend::hir::{
    definitions::{ComponentMemberDeclaration, HirDeclaration, SpecializedComponent},
    types::{HirType, TypesModule},
    TypeId,
};

use crate::{
    ir::temp::TempIRData, IRError, IRPointer, IRSpecializedComponent, IRType, Instruction, SlynxIR,
    Value,
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
        name: TypeId,
        values: &[ComponentMemberDeclaration],
        temp: &mut TempIRData,
    ) -> Result<Value, IRError> {
        let mut vals = Vec::with_capacity(values.len());
        for value in values {
            match value {
                ComponentMemberDeclaration::Property { value, .. } => {
                    let Some(value) = value else {
                        unimplemented!(
                            "Must refactor HIR. Default values should be provided as such they were normal files"
                        );
                    };
                    let value = self.get_value_for(value, temp)?;
                    let value = self.get_value(value);
                    vals.push(value);
                }
                ComponentMemberDeclaration::Child { name, values, .. } => {
                    vals.push(self.get_component_expression(*name, values, temp)?);
                }
                _ => {
                    unimplemented!("Specialized components, not i");
                }
            }
        }
        let vals = self.insert_values(&vals);
        let ty = temp.get_type(name)?;
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
        tys: &TypesModule,
        props: &[ComponentMemberDeclaration],
        temp: &mut TempIRData,
    ) -> Result<(), IRError> {
        {
            //initializes the type
            let component_type = self.get_ir_type(&decl.ty, temp, tys)?;
            let IRType::Component(cid) = self.types.get_type(component_type) else {
                unreachable!(
                    "Something errored that type of component simply isnt Component on ir"
                );
            };

            let Some(HirType::Component { props: ty_props }) = tys.get_component(&decl.ty) else {
                unreachable!("{:?} should map to an Component, but it doesn't", decl);
            };

            for (_, _, prop) in ty_props {
                let ty = self.get_ir_type(prop, temp, tys)?;
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
                ComponentMemberDeclaration::Child { name, values, .. } => {
                    let child = self.get_component_expression(*name, values, temp)?;
                    ir_values.push(child);
                }
                ComponentMemberDeclaration::Specialized(a) => {
                    let spec = self.get_specialized_component_value(a, temp)?;
                    ir_values.push(spec);
                }
            }
        }

        // Armazena os children (ir_values) no componente
        let comp_ptr = temp.get_component(decl.id).ptr;
        self.components[comp_ptr.ptr()].values = self.insert_values(&ir_values);

        Ok(())
    }

    ///Gets a value for specialized components
    pub(crate) fn get_specialized_component_value(
        &mut self,
        spec: &SpecializedComponent,
        temp: &mut TempIRData,
    ) -> Result<Value, IRError> {
        let value = match spec {
            SpecializedComponent::Text { text } => {
                let v = self.get_value_for(text, temp)?;
                let specialized = self.insert_specialized(IRSpecializedComponent::Text(v));
                Value::Specliazed(specialized)
            }
            SpecializedComponent::Div { children } => {
                let children = self.get_component_children(children, temp)?;
                let children = self.insert_values(&children);
                let specialized = self.insert_specialized(IRSpecializedComponent::Div(children));
                Value::Specliazed(specialized)
            }
        };
        Ok(value)
    }

    ///Gets the children values on the given `children` declarations. Note that this will ignore any property, if its got some
    pub(crate) fn get_component_children(
        &mut self,
        children: &[ComponentMemberDeclaration],
        temp: &mut TempIRData,
    ) -> Result<Vec<Value>, IRError> {
        let mut children_values = Vec::with_capacity(children.len());
        for child in children {
            match child {
                ComponentMemberDeclaration::Property { .. } => {} //not handled
                ComponentMemberDeclaration::Child { name, values, .. } => {
                    let child = self.get_component_expression(*name, values, temp)?;
                    children_values.push(child);
                }
                ComponentMemberDeclaration::Specialized(v) => {
                    children_values.push(self.get_specialized_component_value(v, temp)?);
                }
            }
        }
        Ok(children_values)
    }
}
