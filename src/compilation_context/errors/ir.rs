use slynx_codegen::CodegenError;
use slynx_hir::{SlynxHir, id::AnyLocalDeclarationId};

use crate::{
    SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_ir},
};

pub fn format_ir_generation_error(error: &CodegenError, hir: &SlynxHir) -> String {
    match error {
        CodegenError::UnrecognizedVariable(id) => {
            format!("IR internal error: variable id {id:?} is not recognized by the IR",)
        }
        CodegenError::DeclarationNotRecognized(id) => {
            let file = hir.get_file(id.file_id);
            let ty = match id.local_id {
                AnyLocalDeclarationId::Alias(a) => file[a].ty,
                AnyLocalDeclarationId::Component(c) => file[c].ty,
                AnyLocalDeclarationId::Function(f) => file[f].ty,
                AnyLocalDeclarationId::Object(o) => file[o].ty,
                AnyLocalDeclarationId::Static(s) => file[s].ty,
                AnyLocalDeclarationId::Style(s) => file[s].ty,
            };

            hir.view(ty).name()
        }
        CodegenError::IRTypeNotRecognized(id) => {
            format!(
                "IR internal error: type '{}' is not recognized by the IR",
                hir.view(*id).name()
            )
        }
        CodegenError::InternalError(msg) => format!("IR internal error: {msg}"),
    }
}
impl SlynxContext {
    pub fn build_ir_generation_error(&self, error: &CodegenError, hir: &SlynxHir) -> SlynxError {
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
            format_ir_generation_error(error, hir),
            self.file_name(),
            source_code,
            suggestions_from_ir(error),
        )
    }
}
