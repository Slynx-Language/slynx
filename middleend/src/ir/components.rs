use frontend::hir::{
    TypeId,
    definitions::ComponentMemberDeclaration,
    types::{HirType, TypesModule},
};

use crate::{
    Component, IRError, IRPointer, IRType, Instruction, SlynxIR, Value, ir::temp::TempIRData,
};

impl SlynxIR {
    /// Inserts the contents of the provided `decl` into an IR struct type asserting it's a slynx component. This is made because component can be lowered to the equivalent of a struct with methods, thus 'classes'.
    /// The thing is that this is made interanlly with the minimum of abstraction as possible, so it becomes a struct and the components as well as methods are inserted directly into the struct as fields
    pub(crate) fn insert_component_fields_for(
        &mut self,
        decl: TypeId,
        temp: &mut TempIRData,
        tys: &TypesModule,
    ) -> Result<(), IRError> {
        let component_type = self.get_ir_type(&decl, temp, tys)?;
        let IRType::Component(cid) = self.types.get_type(component_type) else {
            unreachable!("Something errored that type of component simply isnt Component on ir");
        };

        let Some(HirType::Component { props: ty_props }) = tys.get_component(&decl) else {
            unreachable!("{:?} should map to an Component, but it doesn't", decl);
        };

        for (_, _, prop) in ty_props {
            let ty = self.get_ir_type(prop, temp, tys)?;
            let comp_ty = self.types.get_component_type_mut(cid);
            comp_ty.insert_field(ty);
        }
        Ok(())
    }

    pub fn get_component_expression(
        &mut self,
        name: TypeId,
        values: &[ComponentMemberDeclaration],
        temp: &mut TempIRData,
    ) -> Result<IRPointer<Value>, IRError> {
        let mut vals = Vec::with_capacity(values.len());
        for value in values {
            match value {
                ComponentMemberDeclaration::Property { value, .. } => {
                    let Some(value) = value else {
                        unimplemented!(
                            "Must refactor HIR. Default values should be provided as such they were normal files"
                        );
                    };
                    vals.push(self.get_value_for(&value, temp)?);
                }
                ComponentMemberDeclaration::Child { name, values, .. } => {
                    vals.push(
                        self.get_component_expression(*name, &values, temp)?
                            .with_length(),
                    );
                }
                _ => {
                    unimplemented!("Specialized components, not i");
                }
            }
        }
        let ptr = IRPointer::new(self.values.len(), vals.len());
        for value in vals {
            let value = self.get_value(value);
            self.insert_value(value);
        }
        let ty = temp.get_type(name)?;
        let instruction =
            self.insert_instruction(temp.current_label(), Instruction::component(ty, ptr));
        Ok(self
            .insert_value(Value::Instruction(instruction))
            .with_length())
    }

    pub fn initialize_component(
        &mut self,
        _: IRPointer<Component, 1>,
        props: &[ComponentMemberDeclaration],
        _temp: &mut TempIRData,
    ) -> Result<(), IRError> {
        //let component = self.get_component_mut(comp);
        for prop in props {
            match prop {
                ComponentMemberDeclaration::Property { .. } => {
                    //already implemented on insert_component_fields
                }
                ComponentMemberDeclaration::Child { name, values, span } => {
                    let component = self.get_component_mut(comp.clone());
                    let child = self.get_component_expression(*name, values, temp);
                }
                ComponentMemberDeclaration::Specialized(_) => {}
            }
        }
        Ok(())
    }
}
