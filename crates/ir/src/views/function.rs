use crate::{Function, IRFunction, IRStorage, IRType, IRTypeId, IRViewer, Label, Value};

impl<'a> IRViewer<'a, Label> {
    pub fn get_name(&self) -> &str {
        self.ir.strings.get_name(self.ir.get(self.ptr).name())
    }
    pub fn get_instructions(&self) -> &[Value] {
        &self.ir.impure_instructions[self.value().instruction_range()]
    }
}

impl<'a> std::ops::Deref for IRViewer<'a, Label> {
    type Target = Label;
    fn deref(&self) -> &Self::Target {
        self.value()
    }
}

impl<'a> IRViewer<'a, Function> {
    pub fn raw_type(&self) -> &IRFunction {
        let ty = self.ir.get(self.ptr).ty();
        let IRType::Function(id) = self.ir.types.get_type(ty) else {
            unreachable!();
        };
        self.ir.types.get_function_type(id)
    }

    pub fn get_name(&self) -> &str {
        let name = self.ir.get(self.ptr).name();
        self.ir.strings.get_name(name)
    }

    pub fn get_arguments(&self) -> &[IRTypeId] {
        self.raw_type().get_args()
    }
    pub fn get_return_type(&self) -> IRTypeId {
        self.raw_type().get_return_type()
    }
    pub fn get_labels(&self) -> Vec<IRViewer<'a, Label>> {
        self.ir
            .get(self.ptr)
            .labels_ptr()
            .iter()
            .map(|ptr| IRViewer { ptr, ir: self.ir })
            .collect()
    }
}
