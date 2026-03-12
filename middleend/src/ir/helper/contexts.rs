use crate::{IRType, IRTypeId, SlynxIR, ir::model::{Context, IRPointer, IRVar, Instruction, Label}};

impl SlynxIR {
    ///Returns the return type of the given context `ir`.
    pub fn return_type_of_context(&self, ir: IRPointer<Context>) -> IRTypeId {
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
    pub fn get_labels_of(&self, ir: IRPointer<Context>) -> &[Label] {
        let ptr = self.contexts[ir.ptr()].labels_ptr();
        &self.labels[ptr.ptr()..ptr.len()]
    }

    ///Gets the labels of the given context `ir`.
    pub fn get_labels_mut_of(&mut self, ir: IRPointer<Context>) -> &mut [Label] {
        let ptr = self.contexts[ir.ptr()].labels_ptr();
        &mut self.labels[ptr.ptr()..ptr.len()]
    }
    
    ///Creates a new label and returns its pointer.
    pub(self) fn create_label(&mut self) -> IRPointer<Label> {
        let ptr = self.labels.len();
        self.labels.push(Label::new());
        IRPointer::new(ptr, 1)
    }
    
    ///Inserts a new label into the given context and returns its pointer. Determines for the label to have the provided `label` name.
    pub fn insert_label(&mut self, ir: IRPointer<Context>, _label: &str) -> IRPointer<Label> {
        self.contexts[ir.ptr()].insert_label(); //this just increases the label count on the context
        self.create_label()
    }
    
    ///Inserts a new variable into the given context and returns its pointer.
    pub fn insert_variable(&mut self, ir: IRPointer<Context>, value: IRPointer<Instruction>) -> IRPointer<IRVar> {
        let ptr = self.variables.len();
        self.variables.push(IRVar::new(value));
        let out = IRPointer::new(ptr, 1);
        if let Some(last) = self.get_labels_mut_of(ir).last_mut() {
            last.insert_variable();
            out
        } else {
            unreachable!("A context when inserting a new variable should have at least one label")
        }
    }
}