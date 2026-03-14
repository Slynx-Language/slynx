use std::collections::HashMap;

use frontend::hir::{TypeId, VariableId};

use crate::{
    IRTypeId,
    ir::model::{Context, IRPointer, IRVar},
};

///Temporary IR Data to be able to map the HIR contents to the IR contents that are being generated. This should only live during `generate` function of
/// slynx ir
pub struct TempIRData {
    ///Maps HIR types to IR types
    types_mapping: HashMap<TypeId, IRTypeId>,
    ///Maps HIR functions to IR functions
    functions: HashMap<frontend::hir::DeclarationId, IRPointer<Context, 1>>,
    ///The current function being generated
    current_function: IRPointer<Context, 1>,
    ///The variables on the current function
    variables: Vec<(VariableId, IRPointer<IRVar, 1>)>,
}

impl TempIRData {
    pub fn new() -> Self {
        Self {
            types_mapping: HashMap::new(),
            functions: HashMap::new(),
            current_function: IRPointer::null(),
            variables: Vec::new(),
        }
    }

    #[inline]
    ///Gets the variable that matches the provided `id` on the current function
    pub fn get_variable(&self, id: VariableId) -> Option<IRPointer<IRVar, 1>> {
        self.variables
            .iter()
            .find_map(|p| if p.0 == id { Some(p.1.clone()) } else { None })
    }

    ///Maps the provided `hty`(hir type) to the provided `ity`(ir type)
    #[inline]
    pub fn define_type(&mut self, hty: TypeId, ity: IRTypeId) {
        self.types_mapping.insert(hty, ity);
    }

    #[inline]
    ///Maps the provided `fid`(hir function id) to the provided `func`(ir function)
    pub fn map_function(&mut self, fid: frontend::hir::DeclarationId, func: IRPointer<Context, 1>) {
        self.functions.insert(fid, func);
    }

    #[inline]
    ///Maps the provided `fid`(hir function id) to the provided `func`(ir function)
    pub fn get_function(&self, fid: frontend::hir::DeclarationId) -> IRPointer<Context, 1> {
        self.functions
            .get(&fid)
            .cloned()
            .expect("For some reason the provided Function Id is not declared")
    }

    #[inline]
    ///Gets the IR type for the provided `ty`(hir type)
    pub fn get_type(&self, ty: TypeId) -> IRTypeId {
        self.types_mapping
            .get(&ty)
            .cloned()
            .expect("For some reason the provided HIR type was not defined")
    }

    #[inline]
    ///Sets the current function being generated and resets the variables
    pub fn set_current_function(&mut self, func: IRPointer<Context, 1>) {
        self.variables.clear();
        self.current_function = func;
    }
}
