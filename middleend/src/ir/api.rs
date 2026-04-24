use either::Either::{Left, Right};

use crate::{
    Component, Context, ControlFlowGraph, IRPointer, IRTypes, Instruction, Label, Operand, SlynxIR,
    Value, ir::instructions::InstructionPtr,
};

impl SlynxIR {
    pub fn generate_context_cfg(&self, context: &Context) -> ControlFlowGraph {
        let labels = self.get_context_labels(context);
        let mut initial = ControlFlowGraph::new();
        for label in labels {}

        initial
    }

    pub fn contexts(&self) -> &[Context] {
        &self.contexts
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }

    pub fn get_context_labels(&self, ctx: &Context) -> &[Label] {
        let ptr = ctx.labels_ptr();
        &self.labels[ptr.range()]
    }

    pub fn get_label_instructions(&self, label: &Label) -> Vec<&[Instruction]> {
        let ptr = label.instructions();
        self.get_multiple_instructions(ptr)
    }

    pub fn get_label_instructions_with_ptrs(
        &self,
        label: &Label,
    ) -> Vec<(&[Instruction], IRPointer<Instruction>)> {
        let ptr = label.instructions();
        self.get_multiple_instructions_and_ptrs(ptr)
    }
    ///Retrieves the inner struct that manages the types on the IR
    pub fn ir_types(&self) -> &IRTypes {
        &self.types
    }

    ///Retrieves a `operand` array that is pointed by the given ptr
    pub fn get_operand_by_pointer(&self, ptr: IRPointer<Operand>) -> &[Operand] {
        &self.operands[ptr.range()]
    }
    ///Retrieves a `value` array that is pointed by the given ptr
    pub fn get_values_by_pointer(&self, ptr: IRPointer<Value>) -> &[Value] {
        &self.values[ptr.range()]
    }

    ///Retrieves all the instructions that are pointer by the given `ptr`, since its the same as **instruction, this returns a vector containing the instructions returned by each
    ///pointer**
    pub fn get_multiple_instructions(
        &self,
        ptr: IRPointer<IRPointer<Instruction>>,
    ) -> Vec<&[Instruction]> {
        let ptrs = &self.instruction_pointers[ptr.range()];
        let mut out = Vec::with_capacity(ptrs.len());
        for ptr in ptrs {
            out.push(&self.instructions[ptr.range()]);
        }
        out
    }
    ///Retrieves all the instructions that are pointer by the given `ptr`, since its the same as **instruction, this returns a vector containing the instructions returned by each
    ///pointer
    pub fn get_multiple_instructions_and_ptrs(
        &self,
        ptr: IRPointer<IRPointer<Instruction>>,
    ) -> Vec<(&[Instruction], IRPointer<Instruction>)> {
        let ptrs = &self.instruction_pointers[ptr.range()];
        let mut out = Vec::with_capacity(ptrs.len());
        for ptr in ptrs {
            out.push((&self.instructions[ptr.range()], *ptr));
        }
        out
    }
    ///Retriueves the instructions pointed by the given `ptr`
    pub fn get_instruction_by_pointer(&self, ptr: IRPointer<Instruction>) -> &[Instruction] {
        &self.instructions[ptr.range()]
    }

    #[inline]
    pub fn dereference_instruction_ptr(&self, ptr: InstructionPtr) -> IRPointer<Instruction> {
        match ptr {
            Left(ptr) => self.instruction_pointers[ptr.ptr()].clone(),
            Right(e) => e,
        }
    }
}
