use slynx_codegen::{CodegenError, CodegenErrorKind};
use slynx_hir::{SlynxHir, id::AnyLocalDeclarationId};

use crate::{
    ErrorPosition, SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_ir},
};

pub fn format_ir_generation_error(error: &CodegenError, hir: &SlynxHir) -> String {
    match &error.kind {
        CodegenErrorKind::NotAStruct(id) => {
            format!(
                "IR internal error: '{}' is not a struct but was trying to be used as one",
                hir.view(*id).pretty_name()
            )
        }
        CodegenErrorKind::NotAnEnum(id) => {
            format!(
                "IR internal error: '{}' is not an enum but was trying to be used as one",
                hir.view(*id).pretty_name()
            )
        }
        CodegenErrorKind::InvalidVariantIndex(id, index) => {
            format!(
                "IR internal error: variant index {index} is out of bounds for enum '{}'",
                hir.view(*id).pretty_name()
            )
        }
        CodegenErrorKind::MissingEnumLayout(id) => {
            format!(
                "IR internal error: the layout of enum '{}' is not registered",
                hir.view(*id).pretty_name()
            )
        }
        CodegenErrorKind::MissingEnumPayload(id) => {
            format!(
                "IR internal error: enum '{}' is missing part of its payload layout",
                hir.view(*id).pretty_name()
            )
        }
        CodegenErrorKind::IRError(error) => format!("IR internal error: {error}"),

        CodegenErrorKind::UnrecognizedVariable(id) => {
            format!(
                "IR internal error: variable '{}' is not recognized by the IR",
                hir.get_variable_name(*id)
            )
        }
        CodegenErrorKind::DeclarationNotRecognized(id) => {
            let file = hir.get_file(id.owner);
            let ty = match id.term {
                AnyLocalDeclarationId::Alias(a) => file[a].ty,
                AnyLocalDeclarationId::Component(c) => file[c].ty,
                AnyLocalDeclarationId::Function(f) => file[f].ty,
                AnyLocalDeclarationId::Object(o) => file[o].ty,
                AnyLocalDeclarationId::Static(s) => file[s].ty,
                AnyLocalDeclarationId::Enum(e) => file[e].ty,
            };

            hir.view(ty).pretty_name()
        }
        CodegenErrorKind::IRTypeNotRecognized(id) => {
            format!(
                "IR internal error: type '{}' is not recognized by the IR",
                hir.view(*id).pretty_name()
            )
        }
    }
}
impl SlynxContext {
    pub fn build_ir_generation_error(&self, error: CodegenError, hir: &SlynxHir) -> SlynxError {
        let info = self.get_line_info(&self.entry_point, error.span.start as usize);
        let source_code = self
            .get_entry_point_source()
            .lines()
            .next()
            .unwrap_or("Internal IR generation error")
            .to_string();

        SlynxError::new_compiler(
            ErrorPosition {
                line: info.line,
                column: info.column_start,
                end_column: info.column_end,
            },
            format_ir_generation_error(&error, hir),
            self.file_name(),
            source_code,
            suggestions_from_ir(&error),
            error.backtrace,
        )
    }
}
