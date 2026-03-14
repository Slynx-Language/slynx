use crate::{
    IRType, IRTypeId, SlynxIR,
    ir::model::{Context, IRPointer, Label, Value},
};

impl SlynxIR {
    #[inline]
    ///Retrieves the context from its provided `ctx`
    pub(crate) fn get_context(&self, ctx: IRPointer<Context, 1>) -> &Context {
        &self.contexts[ctx.ptr()]
    }

    #[inline]
    ///Retrieves the context from its provided `ctx`
    pub(crate) fn get_context_mut(&mut self, ctx: IRPointer<Context, 1>) -> &Context {
        &mut self.contexts[ctx.ptr()]
    }
    ///Returns the return type of the given context `ir`.
    pub fn return_type_of_context(&self, ir: IRPointer<Context, 1>) -> IRTypeId {
        let ctx = self.get_context(ir);
        let id = ctx.ty();
        match self.types.get_type(id) {
            IRType::Function(func) => {
                let func = self.types.get_function_type(func);
                func.get_return_type()
            }
            _ => unreachable!("Type of function should be Function on the IR"),
        }
    }

    ///Gets the labels of the given context `ir`.
    pub fn get_labels_of(&self, ir: IRPointer<Context, 1>) -> &[Label] {
        let ptr = self.contexts[ir.ptr()].labels_ptr();
        &self.labels[ptr.ptr()..ptr.len()]
    }

    ///Gets the labels of the given context `ir`.
    pub fn get_labels_mut_of(&mut self, ir: IRPointer<Context, 1>) -> &mut [Label] {
        let ptr = self.contexts[ir.ptr()].labels_ptr();
        &mut self.labels[ptr.ptr()..ptr.len()]
    }

    ///Creates a new label and returns its pointer.
    pub(self) fn create_label(&mut self) -> IRPointer<Label, 1> {
        let ptr = self.labels.len();
        self.labels.push(Label::new());
        IRPointer::new(ptr, 1)
    }

    ///Inserts a new label into the given context and returns its pointer. Determines for the label to have the provided `label` name.
    pub fn insert_label(&mut self, ir: IRPointer<Context, 1>, _label: &str) -> IRPointer<Label, 1> {
        self.contexts[ir.ptr()].insert_label(); //this just increases the label count on the context
        self.create_label()
    }

    pub fn insert_value(&mut self, value: Value) -> IRPointer<Value, 1> {
        let ptr = self.values.len();
        self.values.push(value);
        let out = IRPointer::new(ptr, 1);
        out
    }
}
