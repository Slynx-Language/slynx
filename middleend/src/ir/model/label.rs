use common::SymbolPointer;
use smallvec::SmallVec;

use crate::{IRTypeId, Value};

use super::{IRPointer, instruction::Instruction};

#[derive(Debug, Clone)]
///A label is a named 'piece' of block that has got instructions and can be used to determine values
pub struct Label {
    name: SymbolPointer,
    ///The instructions this label has got. The max limit due to the IRPointer is about 65k instructions per label. The idea is that, the label will filter only the values that must be read
    instruction: IRPointer<IRPointer<Instruction>>,
    ///Type of the arguments
    arguments: SmallVec<[IRTypeId; 2]>,
}

impl Label {
    ///Creates a new empty label
    pub fn new(name: SymbolPointer) -> Self {
        Self {
            name,
            instruction: IRPointer::null(),
            arguments: SmallVec::new(),
        }
    }

    pub fn insert_arguments(&mut self, arguments: &[IRTypeId]) {
        for arg in arguments {
            self.arguments.push(*arg);
        }
    }

    #[inline]
    ///Returns the label's instruction pointer
    pub fn instruction(&self) -> IRPointer<IRPointer<Instruction>> {
        self.instruction.clone()
    }
    #[inline]
    ///Inserts an instruction into the label's instruction list
    pub fn insert_instruction(&mut self) {
        self.instruction.increase_length();
    }

    #[inline]
    ///Returns the argument types (parameters) of this label
    pub fn arguments(&self) -> &[IRTypeId] {
        &self.arguments
    }

    #[inline]
    ///Adds a parameter type to this label
    pub fn add_argument(&mut self, ty: IRTypeId) {
        self.arguments.push(ty);
    }

    ///Gets the `index`('th) argument of this label
    pub fn get_argument_value(&self, index: usize) -> Value {
        debug_assert!(index < self.arguments.len());
        Value::LabelArg(index)
    }

    ///Gets the name of this label
    pub fn name(&self) -> SymbolPointer {
        self.name
    }
}
