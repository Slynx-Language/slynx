use frontend::hir::definitions::ComponentMemberDeclaration;

use crate::{Component, IRError, IRPointer, SlynxIR, ir::temp::TempIRData};

impl SlynxIR {
    pub fn initialize_component(
        &mut self,
        _: IRPointer<Component, 1>,
        props: &[ComponentMemberDeclaration],
        _temp: &mut TempIRData,
    ) -> Result<(), IRError> {
        //let component = self.get_component_mut(comp);
        for prop in props {
            match prop {
                ComponentMemberDeclaration::Property { .. } => {}
                ComponentMemberDeclaration::Child { .. } => {}
                ComponentMemberDeclaration::Specialized(_) => {}
            }
        }
        Ok(())
    }
}
