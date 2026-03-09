mod context;
mod instruction;
mod temp;
mod label;
mod ptr;

pub use context::*;
pub use ptr::*;
pub use label::*;
pub use instruction::*;

use frontend::hir::{definitions::HirDeclaration, types::TypesModule};


use crate::{IRTypeId, IRTypes, ir::{
    instruction::{Instruction, Operand},
    label::Label, temp::TempIRData,
}};

///All the IR containing contexts, labels, instructions and operands
pub struct SlynxIR {
    ///The contexts of this IR
    contexts: Vec<Context>,
    labels: Vec<Label>,
    instructions: Vec<Instruction>,
    operands: Vec<Operand>,
    types: IRTypes,
}

impl SlynxIR {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            labels: Vec::new(),
            instructions: Vec::new(),
            operands: Vec::new(),
            types: IRTypes::new(),
        }
    }
    
    fn create_blank_function(&mut self, args: &[IRTypeId], ret: IRTypeId) -> IRPointer<Context> {
        let context = Context::new();
        let ptr = self.contexts.len();
        self.contexts.push(context);
        IRPointer::new(ptr, 1)
    }
    
    pub fn generate(&mut self, hir: Vec<HirDeclaration>, tys: TypesModule) {
        let mut _temp  = TempIRData::new();
        //hoist of the objects
        for declaration in &hir {
            match declaration.kind {
                frontend::hir::definitions::HirDeclarationKind::Object => {
                    let out = self.types.create_empty_struct();
                    debug_assert!(out.0 == declaration.id.as_raw() as usize);
                }
                frontend::hir::definitions::HirDeclarationKind::Function { args, statements, name } => {
                    declaration.ty
                    let out = self.create_context()
                }
                
                _ => {}
            }
        }
        for declaration in hir {
            match declaration.kind {
                frontend::hir::definitions::HirDeclarationKind::Object => {
                    
                }
                _ => {}
            }
        }
    }
}
