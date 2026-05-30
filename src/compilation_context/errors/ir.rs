use std::collections::HashMap;

use common::SymbolPointer;
use slynx_codegen::CodegenError;
use slynx_hir::{
    SlynxHir, VariableId,
    modules::{DeclarationsModule, TypesModule},
};
use slynx_ir::SlynxIR;

use crate::{
    SlynxContext,
    compilation_context::{
        errors::{SlynxError, helpers::suggestions_from_ir},
        format_ir_generation_error,
    },
};

impl SlynxContext {
    pub fn build_ir_generation_error(
        &self,
        error: &CodegenError,
        variable_names: &HashMap<VariableId, SymbolPointer<SlynxHir>>,
        types_module: &SlynxHir,
        declarations_module: &DeclarationsModule,
    ) -> SlynxError {
        let source_code = self
            .get_entry_point_source()
            .lines()
            .next()
            .unwrap_or("Internal IR generation error")
            .to_string();

        SlynxError::new_hir(
            0,
            0,
            0,
            format_ir_generation_error(error, variable_names, types_module, declarations_module),
            self.file_name(),
            source_code,
            suggestions_from_ir(error),
        )
    }
}
