use crate::{Function, IRPointer, IRStorage, IRType, IRTypeId, Instruction, Label, SlynxIR};

pub struct FunctionBuilder<'a> {
    ir: &'a mut SlynxIR,
    func_id: IRPointer<Function, 1>,
    results: Vec<LabelResult>,
}

pub struct LabelResult {
    label: IRPointer<Label, 1>,
    instructions: Vec<Instruction>,
}

pub struct LabelBuilder<'a> {
    label: IRPointer<Label, 1>,
    instructions: Vec<Instruction>,
    owner: &'a mut FunctionBuilder<'a>,
}

impl<'a> LabelBuilder<'a> {
    pub fn add_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn generate(self) -> &'a mut FunctionBuilder<'a> {
        self.owner.results.push(LabelResult {
            label: self.label,
            instructions: self.instructions,
        });
        self.owner
    }
}

impl<'a> FunctionBuilder<'a> {
    pub fn new(func_id: IRPointer<Function, 1>, ir: &'a mut SlynxIR) -> Self {
        Self {
            ir,
            func_id,
            results: Vec::new(),
        }
    }

    ///Sets the function type to be the given `ty`. if it isnt a Function type then returns error
    pub fn set_function_type(&mut self, ty: IRTypeId) -> Result<(), ()> {
        let IRType::Function(_) = self.ir.types.get_type(ty) else {
            return Err(());
        };
        *self.ir.get_mut(self.func_id).ty_mut() = ty;
        Ok(())
    }

    pub fn create_label(&'a mut self, name: &str) -> LabelBuilder<'a> {
        let label = self.ir.insert_label(self.func_id, name);
        LabelBuilder {
            instructions: Vec::new(),
            label: label,
            owner: self,
        }
    }

    pub fn generate(self) -> IRPointer<Function, 1> {
        {
            let Some(first) = self.results.first() else {
                return self.func_id;
            };
            self.ir
                .get_mut(self.func_id)
                .set_label_ptr(first.label.with_runtime_length(self.results.len())); //defines where its label pointer initializes and ends.
        }

        let mut initial_ptr = self.ir.get_next_mapeable_instruction_ptr();
        for label_result in self.results.into_iter() {
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
