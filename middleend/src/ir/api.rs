use either::Either::{Left, Right};

use crate::{
    Context, IRPointer, IRTypes, Instruction, Label, Operand, SlynxIR, Value,
    ir::instructions::InstructionPtr,
};

impl SlynxIR {
    pub fn contexts(&self) -> &[Context] {
        &self.contexts
    }

    pub fn get_context_labels(&self, ctx: &Context) -> &[Label] {
        let ptr = ctx.labels_ptr();
        &self.labels[ptr.range()]
    }

    pub fn get_label_instructions(&self, label: &Label) -> &[Instruction] {
        let ptr = label.instruction();
        &self.instructions[ptr.range()]
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

    ///Retrieves a `instruction` array that is pointed by the given ptr. The given `ptr` is a pointer to the valid instructions(mapped ones) a label has
    pub fn get_instructions_by_pointer(
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

    #[inline]
    pub fn dereference_instruction_ptr(&self, ptr: InstructionPtr) -> IRPointer<Instruction> {
        match ptr {
            Left(ptr) => self.instruction_pointers[ptr.ptr()].clone(),
            Right(e) => e,
        }
    }
}
