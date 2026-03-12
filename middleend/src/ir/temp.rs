use std::collections::HashMap;

use frontend::hir::TypeId;

use crate::{IRTypeId, ir::model::{Context, IRPointer}};

///Temporary IR Data to be able to map the HIR contents to the IR contents that are being generated. This should only live during `generate` function of
/// slynx ir
pub struct TempIRData {
    ///Maps HIR types to IR types
    types_mapping: HashMap<TypeId, IRTypeId>,
    ///Maps HIR functions to IR functions
    functions: HashMap<frontend::hir::DeclarationId, IRPointer<Context>>,
    ///The current function being generated
    current_function: IRPointer<Context>
}

impl TempIRData {
    pub fn new() -> Self {
        Self {
            types_mapping: HashMap::new(),
            functions: HashMap::new(),
            current_function: IRPointer::null()
        }
    }
    ///Maps the provided `hty`(hir type) to the provided `ity`(ir type)
    #[inline]
    pub fn define_type(&mut self, hty: TypeId, ity: IRTypeId) {
        self.types_mapping.insert(hty, ity);
    }

    #[inline]
    ///Maps the provided `fid`(hir function id) to the provided `func`(ir function)
    pub fn map_function(&mut self, fid: frontend::hir::DeclarationId, func: IRPointer<Context>) {
        self.functions.insert(fid, func);
    }
    
    #[inline]
    ///Maps the provided `fid`(hir function id) to the provided `func`(ir function)
    pub fn get_function(&self, fid: frontend::hir::DeclarationId) -> IRPointer<Context> {
        self.functions.get(&fid).cloned().expect("For some reason the provided Function Id is not declared")
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
    ///Sets the current function being generated
    pub fn set_current_function(&mut self, func: IRPointer<Context>) {
        self.current_function = func;
    }
    
}
