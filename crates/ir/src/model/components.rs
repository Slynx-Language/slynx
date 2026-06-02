use crate::{IRPointer, IRTypeId, Instruction};

#[derive(Debug, Clone)]
pub struct Component {
    pub(crate) ty: IRTypeId,
    pub(crate) ui_instruction: IRPointer<Instruction>, //should be asserted to be ui operations only
}

impl Component {
    pub fn new(initial_type: IRTypeId, ui_instruction: IRPointer<Instruction>) -> Self {
        Self {
            ty: initial_type,
            ui_instruction,
        }
    }

    pub fn ir_type(&self) -> IRTypeId {
        self.ty
    }

    ///The UI operations this component must handle. In general, before adding the component into the ui
    pub fn instructions(&self) -> IRPointer<Instruction> {
        self.ui_instruction
    }
}
