use crate::{
    Component, ComponentBuilder, ControlFlowGraph, Function, IRPointer, IRTypeId, SlynxIR,
    builder::FunctionBuilder,
};

impl SlynxIR {
    pub fn create_struct(&mut self, name: &str) -> IRTypeId {
        let name = self.strings.intern(name);
        self.create_empty_struct(name).0
    }

    /// Create a new empty function with the given name and return its pointer.
    pub fn create_function(&mut self, name: &str) -> IRPointer<Function, 1> {
        let name = self.strings.intern(name);
        let func = Function::new(name, self.types.create_function_type().0);
        let ptr = self.functions.len();
        self.functions.push(func);
        IRPointer::new(ptr, 1)
    }

    /// Start building the function at `ptr`.
    pub fn build_function<'a>(&'a mut self, ptr: IRPointer<Function, 1>) -> FunctionBuilder<'a> {
        FunctionBuilder::new(ptr, self)
    }

    pub fn create_component(&mut self, name: &str) -> IRPointer<Component, 1> {
        let name = self.strings.intern(name);
        let (type_id, _) = self.create_empty_component(name);
        let component = Component::new(type_id, IRPointer::null());
        let ptr = self.components.len();
        self.components.push(component);
        IRPointer::new(ptr, 1)
    }

    pub fn build_component<'a>(
        &'a mut self,
        component: IRPointer<Component, 1>,
    ) -> ComponentBuilder<'a> {
        ComponentBuilder::new(component, self)
    }

    /// Build a [`ControlFlowGraph`] for `function`.
    pub fn generate_function_cfg(&self, function: &Function) -> ControlFlowGraph {
        let labels = function.labels_ptr().with_length();
        ControlFlowGraph::new(labels, self)
    }
}
