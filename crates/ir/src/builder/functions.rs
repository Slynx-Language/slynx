use std::ops::{Deref, DerefMut};

use crate::{Function, IRPointer, IRStorage, IRType, IRTypeId, Instruction, Label, SlynxIR};

pub struct FunctionBuilder<'a> {
    ir: &'a mut SlynxIR,
    func_id: IRPointer<Function, 1>,
    current_label: usize,
    labels: Vec<LabelBuilder>,
}

pub struct LabelBuilder {
    label: IRPointer<Label, 1>,
    instructions: Vec<Instruction>,
}

impl LabelBuilder {
    pub fn add_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }
}

impl<'a> FunctionBuilder<'a> {
    pub fn new(func_id: IRPointer<Function, 1>, ir: &'a mut SlynxIR) -> Self {
        Self {
            ir,
            func_id,
            current_label: 0,
            labels: Vec::new(),
        }
    }

    pub fn ir(&'a mut self) -> &'a mut SlynxIR {
        self.ir
    }

    ///Sets the function type to be the given `ty`. if it isnt a Function type then returns error
    pub fn set_function_type(&mut self, ty: IRTypeId) -> Result<(), ()> {
        let IRType::Function(_) = self.ir.types.get_type(ty) else {
            return Err(());
        };
        *self.ir.get_mut(self.func_id).ty_mut() = ty;
        Ok(())
    }

    pub fn create_label(&mut self, name: &str) -> LabelBuilder {
        self.current_label += 1;
        let label = self.ir.insert_label(self.func_id, name);
        LabelBuilder {
            instructions: Vec::new(),
            label: label,
        }
    }

    pub fn generate(self) -> IRPointer<Function, 1> {
        {
            let Some(first) = self.labels.first() else {
                return self.func_id;
            };
            self.ir
                .get_mut(self.func_id)
                .set_label_ptr(first.label.with_runtime_length(self.labels.len())); //defines where its label pointer initializes and ends.
        }

        let mut initial_ptr = self.ir.get_next_mapeable_instruction_ptr();
        for label_result in self.labels.into_iter() {
            self.ir
                .get_mut(label_result.label)
                .set_instructions_pointer(initial_ptr.with_length());
            for instruction in label_result.instructions {
                self.ir
                    .insert_instruction(label_result.label, instruction, true);
            }
            initial_ptr = self.ir.get_next_mapeable_instruction_ptr();
        }
        self.func_id
    }
}

impl<'a> Deref for FunctionBuilder<'a> {
    type Target = LabelBuilder;
    fn deref(&self) -> &Self::Target {
        &self.labels[self.current_label]
    }
}
impl<'a> DerefMut for FunctionBuilder<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.labels[self.current_label]
    }
}
