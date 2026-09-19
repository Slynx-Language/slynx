use crate::{Function, GlobalValue, IRPointer, IRTypeId, SlynxIR, Value};

#[derive(Debug, Clone, Copy)]
pub enum InitValue {
    ZeroInit(IRTypeId),
    Constant(Value),
    Lazy(IRPointer<Function>),
}

impl SlynxIR {
    pub fn create_global(&mut self, name: &str, init: InitValue) -> IRPointer<GlobalValue, 1> {
        let name = self.strings.intern(name);
        let id = self.globals.len();
        self.globals.push(GlobalValue {
            initial_value: init,
            name,
        });
        IRPointer::new(id, 1)
    }
}
