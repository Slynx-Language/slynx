use crate::{IRTypeId, IRViewer, Value};

impl<'a> IRViewer<'a, Value> {
    pub fn raw_type(&self) -> IRTypeId {
        self.value().ir_type()
    }
}
