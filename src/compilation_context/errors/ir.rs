use std::collections::HashMap;

use common::SymbolPointer;
use frontend::hir::{
    VariableId,
    modules::{DeclarationsModule, TypesModule},
};
use middleend::{IRError, SlynxIR};

use crate::{
    SlynxContext, SlynxError, SlynxErrorType,
    compilation_context::{errors::helpers::suggestions_from_ir, format_ir_generation_error},
};

impl SlynxContext {
    pub fn build_ir_generation_error(
        &self,
        error: &IRError,
        ir: &SlynxIR,
        variable_names: &HashMap<VariableId, SymbolPointer>,
        types_module: &TypesModule,
        declarations_module: &DeclarationsModule,
    ) -> color_eyre::Report {
        let source_code = self
            .get_entry_point_source()
            .lines()
            .next()
            .unwrap_or("Internal IR generation error")
            .to_string();

        SlynxError {
            line: 1,
            column_start: 1,
            ty: SlynxErrorType::Compilation,
            message: format_ir_generation_error(
                error,
                ir,
                variable_names,
                types_module,
                declarations_module,
            ),
            file: self.file_name(),
            suggestion: suggestions_from_ir(error),
            source_code,
        }
        .into()
    }
}
