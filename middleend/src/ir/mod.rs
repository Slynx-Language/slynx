mod contexts;
mod helper;
mod model;
mod temp;
pub use model::*;

use frontend::hir::{
    definitions::{ComponentMemberDeclaration, HirDeclaration, HirDeclarationKind},
    symbols::SymbolsModule,
    types::TypesModule,
};

use crate::{BUILTIN_TYPES, IRError, IRTypes};

use temp::TempIRData;

#[derive(Debug)]
///All the IR containing contexts, labels, instructions and operands
pub struct SlynxIR {
    ///The contexts of this IR
    contexts: Vec<Context>,
    labels: Vec<Label>,
    instructions: Vec<Instruction>,
    operands: Vec<Operand>,
    values: Vec<Value>,
    types: IRTypes,
}

impl SlynxIR {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            labels: Vec::new(),
            instructions: Vec::new(),
            operands: Vec::new(),
            values: Vec::new(),
            types: IRTypes::new(),
        }
    }

    pub fn generate(
        &mut self,
        hir: Vec<HirDeclaration>,
        tys: &TypesModule,
        symbols: &SymbolsModule,
    ) -> Result<(), IRError> {
        let mut temp = TempIRData::new();
        //hoist of the objects
        for declaration in &hir {
            match &declaration.kind {
                frontend::hir::definitions::HirDeclarationKind::Object => {
                    let out = self.types.create_empty_struct();

                    temp.define_type(declaration.ty, out);
                    debug_assert_eq!(
                        out.0 - BUILTIN_TYPES.len(),
                        declaration.id.as_raw() as usize
                    );
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
                    debug_assert!(out.0 - BUILTIN_TYPES.len() == declaration.id.as_raw() as usize);
                }
            }
        }
        for declaration in hir {
            match declaration.kind {
                HirDeclarationKind::Object => {
                    self.insert_object_fields_for(declaration.ty, &temp, &tys)?;
                }
                HirDeclarationKind::Function {
                    args, statements, ..
                } => {
                    self.insert_function_type_for(declaration.ty, &temp, &tys)?;
                    let func = temp.get_function(declaration.id);
                    debug_assert!(func.len() == 1);
                    self.initialize_function(
                        func.with_length::<1>(),
                        &statements,
                        &args,
                        &mut temp,
                    )?;
                }
                HirDeclarationKind::ComponentDeclaration { props } => {
                    self.insert_component_fields_for(declaration.ty, &temp, &tys)?;
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
        Ok(())
    }
}
