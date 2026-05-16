use crate::{Context, IRPointer, IRTypeId, Instruction, Value};

#[derive(Debug, Clone)]
pub struct Component {
    pub(crate) ty: IRTypeId,
    pub(crate) values: IRPointer<Value>,
    pub(crate) ui_instruction: IRPointer<Instruction>, //should be asserted to be ui operations only
    pub(crate) init_func: Option<IRPointer<Context, 1>>,
}

impl Component {
    pub fn new(
        initial_type: IRTypeId,
        values: IRPointer<Value>,
        ui_instruction: IRPointer<Instruction>,
    ) -> Self {
        Self {
            ty: initial_type,
            values,
            ui_instruction,
            init_func: None,
        }
    }

    pub fn ir_type(&self) -> IRTypeId {
        self.ty
    }

    ///The default values of the component
    pub fn values(&self) -> IRPointer<Value> {
        self.values
    }
    ///The UI operations this component must handle. In general, before adding the component into the ui
    pub fn instructions(&self) -> IRPointer<Instruction> {
        self.ui_instruction
    }
}
