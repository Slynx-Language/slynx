mod api;
mod components;
mod contexts;
mod helper;
mod instructions;
mod model;
mod temp;
mod visualize;

use common::SymbolsModule;
pub use model::*;
pub use visualize::*;

use frontend::hir::{
    model::{HirDeclaration, HirDeclarationKind, HirType},
    modules::TypesModule,
};

use crate::{BUILTIN_TYPES, IRError, IRTypes};

use temp::TempIRData;

#[derive(Debug)]
///All the IR containing contexts, labels, instructions and operands
pub struct SlynxIR {
    ///The contexts of this IR
    contexts: Vec<Context>,
    ///The Components of this IR
    components: Vec<Component>,
    specialized: Vec<IRSpecializedComponent>,
    ///The labels of this IR
    labels: Vec<Label>,
    ///The instructions of this IR
    instructions: Vec<Instruction>,
    instruction_pointers: Vec<IRPointer<Instruction>>,
    ///The operands of this IR
    operands: Vec<Operand>,
    ///The values of this IR
    values: Vec<Value>,
    slots: Vec<Slot>,
    types: IRTypes,
    ///Pool of interned strings, accessed via StringHandle indices
    strings: SymbolsModule,
}

impl SlynxIR {
    ///Creates a new empty IR
    pub fn new(symbols: SymbolsModule) -> Self {
        Self {
            components: Vec::new(),
            specialized: Vec::new(),
            contexts: Vec::new(),
            labels: Vec::new(),
            instructions: Vec::new(),
            instruction_pointers: Vec::new(),
            operands: Vec::new(),
            values: Vec::new(),
            slots: Vec::new(),
            types: IRTypes::new(),
            strings: symbols,
        }
    }

    ///Returns a reference to the string pool
    pub fn string_pool(&self) -> &SymbolsModule {
        &self.strings
    }

    ///Generates all the code on the IR, with types, functions, lowerings, etc, based on the provided `hir`. The `tys` is expected to be the types module used by the `hir` during all frontend process, as well as
    ///the `symbols`, to be the symbols module used by the same `hir` during all the frontend process
    pub fn generate(&mut self, hir: Vec<HirDeclaration>, tys: &TypesModule) -> Result<(), IRError> {
        let mut temp = TempIRData::new(tys);
        //hoist of the objects
        for declaration in &hir {
            match &declaration.kind {
                HirDeclarationKind::Object => {
                    let out = self.types.create_empty_struct();
                    temp.define_type(declaration.ty, out);
                    // Also map the inner unnamed Struct TypeId (the `rf` of the Reference)
                    if let HirType::Reference { rf, .. } = tys.get_type(&declaration.ty) {
                        temp.define_type(*rf, out);
                    }
                    debug_assert_eq!(
                        out.0 - BUILTIN_TYPES.len(),
                        declaration.id.as_raw() as usize
                    );
                }
                HirDeclarationKind::Function { name, .. } => {
                    let out = self.create_blank_function(*name).with_length();
                    let ctx = self.get_context(out);
                    temp.define_type(declaration.ty, ctx.ty());
                    temp.map_function(declaration.id, out.with_length());
                }
                HirDeclarationKind::ComponentDeclaration { name, .. } => {
                    let out = self.create_blank_component(*name);
                    let fnc = self.get_component(out);
                    temp.define_type(declaration.ty, fnc.ty);
                    temp.map_component(declaration.id, out);
                }
                HirDeclarationKind::Alias => {}
            }
        }
        for declaration in hir {
            match declaration.kind {
                HirDeclarationKind::Object => {
                    self.insert_object_fields_for(declaration.ty, &temp)?;
                }
                HirDeclarationKind::Function {
                    args, statements, ..
                } => {
                    self.insert_function_type_for(declaration.ty, &temp)?;
                    let func = temp.get_function(declaration.id);
                    debug_assert!(func.len() == 1);
                    self.initialize_function(
                        func.with_length::<1>(),
                        &statements,
                        &args,
                        &mut temp,
                    )?;
                }
                HirDeclarationKind::ComponentDeclaration { ref props, .. } => {
                    self.initialize_component(&declaration, props, &mut temp)?;
                }
                HirDeclarationKind::Alias => {
                    let HirType::Reference { rf, .. } =
                        temp.types_module().get_type(&declaration.ty)
                    else {
                        unreachable!("Declaration type of alias is always reference")
                    };

                    let t = temp.get_type(*rf)?;

                    temp.define_type(declaration.ty, t);
                }
            }
        }
        Ok(())
    }

    /// Produces a Slynx IR textual dump (SIR) following the README syntax.
    ///
    /// This uses the helpers defined in the `visualize` module to format labels and
    /// instructions in the human-readable SIR form described in `middleend/README.md`.
    pub fn format_sir(&self) -> String {
        let fmt = visualize::Formatter::new(
            &self.labels,
            &self.values,
            &self.operands,
            &self.types,
            &self.strings,
        );

        fmt.format_labels(&self.instructions)
    }
}
