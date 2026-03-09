use std::collections::HashMap;

use frontend::hir::TypeId;

use crate::IRTypeId;

///Temporary IR Data to be able to map the HIR contents to the IR contents that are being generated. This should only live during `generate` function of
/// slynx ir
pub struct TempIRData {
    ///Maps HIR types to IR types
    types_mapping: HashMap<TypeId, IRTypeId>,
}

impl TempIRData {
    pub fn new() -> Self {
        Self {
            types_mapping: HashMap::new(),
        }
    }
    ///Maps the provided `hty`(hir type) to the provided `ity`(ir type)
    #[inline]
    pub fn define_type(&mut self, hty: TypeId, ity: IRTypeId) {
        self.types_mapping.insert(hty, ity);
    }

    #[inline]
    pub fn get_type(&self, ty: TypeId) -> IRTypeId {
        self.types_mapping
            .get(&ty)
            .cloned()
            .expect("For some reason the provided HIR type was not defined")
    }
}
