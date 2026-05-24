use std::ops::Deref;

use crate::{api::InstructionPtr, views::IRStorage};
use common::SymbolsModule;
use either::Either;

use crate::{
    Component, Formatter, Function, IRPointer, IRTypes, Instruction, Label, Operand, Slot, Value,
};

#[derive(Debug)]
///All the IR containing contexts, labels, instructions and operands
pub struct SlynxIR {
    ///The contexts of this IR
    pub(crate) functions: Vec<Function>,
    ///The Components of this IR
    pub(crate) components: Vec<Component>,
    ///The labels of this IR
    pub(crate) labels: Vec<Label>,
    ///The instructions of this IR
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) instruction_pointers: Vec<IRPointer<Instruction>>,
    ///The operands of this IR
    pub(crate) operands: Vec<Operand>,
    ///The values of this IR
    pub(crate) values: Vec<Value>,
    pub(crate) slots: Vec<Slot>,
    pub(crate) types: IRTypes,
    ///Pool of interned strings, accessed via StringHandle indices
    pub(crate) strings: SymbolsModule<SlynxIR>,
}

impl SlynxIR {
    ///Creates a new empty IR
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            functions: Vec::new(),
            labels: Vec::new(),
            instructions: Vec::new(),
            instruction_pointers: Vec::new(),
            operands: Vec::new(),
            values: Vec::new(),
            slots: Vec::new(),
            types: IRTypes::new(),
            strings: SymbolsModule::new(),
        }
    }

    ///Inserts the provided `instr` on the given `label`. If `map` is `true` then the label will be able to read it when compiling, thus, otherwise, its just an intermediate instruction.
    ///On `map=true`, is garanteed to be `Left` variant, otherwise `Right`
    pub(crate) fn insert_instruction(
        &mut self,
        label: IRPointer<Label, 1>,
        instr: Instruction,
        map: bool,
    ) -> InstructionPtr {
        let ptr = self.instructions.len();
        self.instructions.push(instr);
        if map {
            let outptr = self.instruction_pointers.len();
            self.instruction_pointers.push(IRPointer::new(ptr, 1));
            let label = self.get_mut(label);
            label.insert_instruction();
            Either::Left(IRPointer::new(outptr, 1))
        } else {
            Either::Right(IRPointer::new(ptr, 1))
        }
    }

    /// Produces a Slynx IR textual dump (SIR) following the README syntax.
    ///
    /// This uses the helpers defined in the `visualize` module to format labels and
    /// instructions in the human-readable SIR form described in `middleend/README.md`.
    pub fn format_sir(&self) -> String {
        let fmt = Formatter::new(self);
        let mut out = fmt.format_types();
        out.push_str(&&fmt.format_functions());
        out
    }

    ///Retrieves the next label pointer
    pub(crate) fn get_next_mapeable_instruction_ptr(&self) -> IRPointer<IRPointer<Instruction>, 1> {
        IRPointer::new(self.instruction_pointers.len(), 1)
    }

    ///Creates a new label and returns its pointer.
    fn create_label(&mut self, name: &str) -> IRPointer<Label, 1> {
        let ptr = self.labels.len();
        let name = self.strings.intern(name);
        self.labels.push(Label::new(name));
        IRPointer::new(ptr, 1)
    }

    ///Inserts a new label into the given context and returns its pointer. Determines for the label to have the provided `label` name.
    pub(crate) fn insert_label(
        &mut self,
        ir: IRPointer<Function, 1>,
        label_name: &str,
    ) -> IRPointer<Label, 1> {
        self.functions[ir.ptr()].insert_label(); //this just increases the label count on the context
        let label_ptr = self.create_label(label_name);
        // Initialize the instruction pointer offset to the current end of instruction_pointers,
        // so instructions inserted later are correctly attributed to this label.
        let next = self.get_next_mapeable_instruction_ptr();
        let mut ptr = next.with_length();
        ptr.set_length(ptr.len() - 1);
        self.get_mut(label_ptr).set_instructions_pointer(ptr);
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

impl Deref for SlynxIR {
    type Target = IRTypes;
    fn deref(&self) -> &Self::Target {
        &self.types
    }
}
