mod contexts;
mod temp;
mod helper;
mod model;
use model::*;

use frontend::hir::{
    TypeId,
    definitions::{ComponentMemberDeclaration, HirDeclaration, HirDeclarationKind},
    types::TypesModule,
};

use crate::{
    IRTypeId, IRTypes,
};
use model::{
        {Instruction, Operand},
        Label, 
};
use temp::TempIRData;

///All the IR containing contexts, labels, instructions and operands
pub struct SlynxIR {
    ///The contexts of this IR
    contexts: Vec<Context>,
    labels: Vec<Label>,
    variables: Vec<IRVar>,
    instructions: Vec<Instruction>,
    operands: Vec<Operand>,
    types: IRTypes,
}

impl SlynxIR {
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            contexts: Vec::new(),
            labels: Vec::new(),
            instructions: Vec::new(),
            operands: Vec::new(),
            types: IRTypes::new(),
        }
    }

    pub fn generate(&mut self, hir: Vec<HirDeclaration>, tys: TypesModule) {
        let mut temp = TempIRData::new();
        //hoist of the objects
        for declaration in &hir {
            match &declaration.kind {
                frontend::hir::definitions::HirDeclarationKind::Object => {
                    let out = self.types.create_empty_struct();
                    temp.define_type(declaration.ty, out);
                    debug_assert!(out.0 == declaration.id.as_raw() as usize);
                }
                frontend::hir::definitions::HirDeclarationKind::Function { .. } => {
                    let out = self.create_blank_function().with_length();
                    let ctx = self.get_context(out.clone());
                    temp.define_type(declaration.ty, ctx.ty());
                    temp.map_function(declaration.id, out.with_length());
                }
                HirDeclarationKind::ComponentDeclaration { .. } => {
                    let out = self.types.create_empty_struct();
                    temp.define_type(declaration.ty, out);
                    debug_assert!(out.0 == declaration.id.as_raw() as usize);
                }
            }
        }
        for declaration in hir {
            match declaration.kind {
                HirDeclarationKind::Object => {
                    self.insert_object_fields_for(declaration.ty, &temp, &tys);
                }
                HirDeclarationKind::Function { args, statements, .. } => {
                    self.insert_function_type_for(declaration.ty, &temp, &tys);
                    let func = temp.get_function(declaration.id);
                    debug_assert!(func.len() == 1);
                    self.initialize_function(func.with_length::<1>(), &statements, &args, &mut temp);
                }
                HirDeclarationKind::ComponentDeclaration { props } => {
                    self.insert_component_fields_for(declaration.ty, &temp, &tys);
                    for prop in props {
                        match prop {
                            ComponentMemberDeclaration::Property { .. } => {}
                            ComponentMemberDeclaration::Specialized(_) => {}
                            ComponentMemberDeclaration::Child { .. } => {}
                        }
                    }
                }
            }
        }
    }
}
