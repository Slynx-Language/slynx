use common::SymbolPointer;

use crate::{
    Component, IRType, IRTypeId, Instruction, SlynxIR,
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
    pub(crate) fn get_context_mut(&mut self, ctx: IRPointer<Context, 1>) -> &mut Context {
        &mut self.contexts[ctx.ptr()]
    }

    ///Retrieves the component from its provided `comp`
    pub(crate) fn get_component(&self, comp: IRPointer<Component, 1>) -> &Component {
        &self.components[comp.ptr()]
    }

    // pub(crate) fn get_component_mut(&mut self, comp: IRPointer<Component, 1>) -> &mut Component {
    //     &mut self.components[comp.ptr()]
    // }

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

    ///Returns the return type of the given context `ir`.
    pub fn arg_types_of_context(&self, ir: IRPointer<Context, 1>) -> &[IRTypeId] {
        let ctx = self.get_context(ir);
        let id = ctx.ty();
        match self.types.get_type(id) {
            IRType::Function(func) => {
                let func = self.types.get_function_type(func);
                func.get_args()
            }
            _ => unreachable!("Type of function should be Function on the IR"),
        }
    }

    ///Retrieves the next label pointer
    pub fn get_next_label_ptr(&self) -> IRPointer<Label, 1> {
        IRPointer::new(self.labels.len(), 1)
    }
    ///Retrieves the next label pointer
    pub fn get_next_mapeable_instruction_ptr(&self) -> IRPointer<IRPointer<Instruction>, 1> {
        IRPointer::new(self.instruction_pointers.len(), 1)
    }
    ///Gets the labels of the given context `ir`.
    pub fn get_labels_of(&self, ir: IRPointer<Context, 1>) -> &[Label] {
        let ptr = self.contexts[ir.ptr()].labels_ptr();
        &self.labels[ptr.range()]
    }

    ///Gets the labels of the given context `ir`.
    pub fn get_labels_mut_of(&mut self, ir: IRPointer<Context, 1>) -> &mut [Label] {
        let ptr = self.contexts[ir.ptr()].labels_ptr();
        &mut self.labels[ptr.range()]
    }

    ///Creates a new label and returns its pointer.
    fn create_label(&mut self, name: SymbolPointer) -> IRPointer<Label, 1> {
        let ptr = self.labels.len();
        self.labels.push(Label::new(name));
        IRPointer::new(ptr, 1)
    }

    ///Inserts a new label into the given context and returns its pointer. Determines for the label to have the provided `label` name.
    pub fn insert_label(&mut self, ir: IRPointer<Context, 1>, label: &str) -> IRPointer<Label, 1> {
        self.contexts[ir.ptr()].insert_label(); //this just increases the label count on the context
        let name = self.strings.intern(label);
        let label_ptr = self.create_label(name);
        // Initialize the instruction pointer offset to the current end of instruction_pointers,
        // so instructions inserted later are correctly attributed to this label.
        let next = self.get_next_mapeable_instruction_ptr();
        let mut ptr = next.with_length();
        ptr.set_length(ptr.len() - 1);
        self.get_label_mut(label_ptr).set_instructions_pointer(ptr);
        label_ptr
    }

    pub fn insert_values(&mut self, value: &[Value]) -> IRPointer<Value> {
        let ptr = self.values.len();
        self.values.extend_from_slice(value);
        IRPointer::new(ptr, value.len())
    }

    ///Inserts the given value and returns its pointer
    pub fn insert_value(&mut self, value: Value) -> IRPointer<Value, 1> {
        let ptr = self.values.len();
        self.values.push(value);
        IRPointer::new(ptr, 1)
    }
}
