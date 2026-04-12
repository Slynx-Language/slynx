use crate::{IRPointer, IRTypeId, Value};

#[derive(Debug, Clone)]
pub struct Component {
    pub(crate) ty: IRTypeId,
    pub(crate) values: IRPointer<Value>,
}

impl Component {
    pub fn new(initial_type: IRTypeId, values: IRPointer<Value>) -> Self {
        Self {
            ty: initial_type,
            values,
        }
    }
}
