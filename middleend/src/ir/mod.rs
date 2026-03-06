mod context;
mod instruction;
mod irtype;
mod label;
mod ptr;
pub use context::*;
use frontend::hir::{definitions::HirDeclaration, types::TypesModule};
pub use ptr::*;

use crate::ir::{
    instruction::{Instruction, Operand},
    irtype::IRType,
    label::Label,
};

///All the IR containing contexts, labels, instructions and operands
pub struct SlynxIR {
    ///The contexts of this IR
    contexts: Vec<Context>,
    labels: Vec<Label>,
    instructions: Vec<Instruction>,
    operands: Vec<Operand>,
    types: Vec<IRType>,
}

impl SlynxIR {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            labels: Vec::new(),
            instructions: Vec::new(),
            operands: Vec::new(),
            types: Vec::new(),
        }
    }
    pub fn generate(&mut self, hir: Vec<HirDeclaration>, tys: TypesModule) {
        //hoist of the objects
        for declaration in hir {
            match declaration.kind {
                frontend::hir::definitions::HirDeclarationKind::Object => {}
            }
        }
    }
}
