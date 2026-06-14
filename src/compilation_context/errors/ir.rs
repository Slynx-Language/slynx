use common::SymbolPointer;
use dashmap::DashMap;
use slynx_codegen::CodegenError;
use slynx_hir::{SlynxHir, VariableId};

use crate::{
    SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_ir},
};

pub fn format_ir_generation_error(
    error: &CodegenError,
    variable_names: &DashMap<VariableId, SymbolPointer<SlynxHir>>,
    hir: &SlynxHir,
) -> String {
    match error {
        CodegenError::UnrecognizedVariable(id) => {
            if let Some(name) = variable_names
                .get(id)
                .map(|v| *v.value())
                .map(|symbol| hir.get_name(symbol))
            {
                format!("IR internal error: variable '{name}' is not recognized by the IR")
            } else {
                format!(
                    "IR internal error: variable id {} is not recognized by the IR",
                    id.as_raw()
                )
            }
        }
        CodegenError::DeclarationNotRecognized(id) => {
            let ty = hir.find_declaration(*id).ty;
            let name = hir.get_name_of_type(ty);
            name.map(|symbol| hir.get_name(symbol))
                .unwrap_or("Unrecognized Declaration? This is a bug")
                .to_string()
        }
        CodegenError::IRTypeNotRecognized(id) => {
            if let Some(name) = hir.get_name_of_type(*id).map(|symbol| hir.get_name(symbol)) {
                format!("IR internal error: type '{name}' is not recognized by the IR")
            } else {
                format!(
                    "IR internal error: type id {} is not recognized by the IR",
                    id.as_raw()
                )
            }
        }
        CodegenError::InternalError(msg) => format!("IR internal error: {msg}"),
    }
}
impl SlynxContext {
    pub fn build_ir_generation_error(
        &self,
        error: &CodegenError,
        variable_names: &DashMap<VariableId, SymbolPointer<SlynxHir>>,
        hir: &SlynxHir,
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
            format_ir_generation_error(error, variable_names, hir),
            self.file_name(),
            source_code,
            suggestions_from_ir(error),
        )
    }
}
