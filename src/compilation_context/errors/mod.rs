use color_eyre::owo_colors::OwoColorize;

pub mod checker;
pub mod helpers;
pub mod hir;
pub mod ir;
pub mod lexer;
pub mod parser;

use helpers::SlynxSuggestion;

#[derive(Debug)]
///The type of the error that was generated
pub enum SlynxErrorType {
    Lexer,
    Parser,
    Hir,
    Type,
    Compilation,
}
impl std::fmt::Display for SlynxErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlynxErrorType::Lexer => write!(f, "Lexing Error"),
            SlynxErrorType::Parser => write!(f, "Parsing Error"),
            SlynxErrorType::Hir => write!(f, "Name Resolution Error"),
            SlynxErrorType::Compilation => write!(f, "Compilation Error"),
            SlynxErrorType::Type => write!(f, "Type Checking Error"),
        }
    }
}

#[derive(Debug)]
///An error that will be shown if something fails
pub struct SlynxError {
    ty: SlynxErrorType,
    line: usize,
    column_start: usize,
    message: String,
    ///The file path the error occuried
    file: String,
    source_code: String,
    suggestion: Vec<SlynxSuggestion>,
}
impl std::error::Error for SlynxError {}

impl std::fmt::Display for SlynxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_error = self.ty.to_string();
        let source = self.source_code.replace("\t", " ");
        let before_error = format!("{} |", self.line);
        let error_with_data = format!("{before_error}{source}");

        let error_points = {
            let points_offset = " ".to_string();
            let points = "^".repeat(self.source_code.trim().len());
            format!("{before_error}{points_offset}{points}",)
        };

        let line_and_column = format!(
            "{}:{}",
            self.line.blue().bold(),
            self.column_start.blue().bold()
        );
        writeln!(
            f,
            "{}: {} => {}:{line_and_column}",
            type_error.green().bold(),
            self.message.red(),
            self.file.bold()
        )?;
        writeln!(f, "{}", error_with_data)?;
        writeln!(f, "{}", error_points)?;
        for suggestion in self.suggestion.iter() {
            writeln!(f, "-->{}", suggestion)?;
        }
        writeln!(f)
    }
}

macro_rules! impl_slynx_error {
    ($name: ident -> $value:expr) => {
        impl SlynxError {
            pub fn $name(
                line: usize,
                column: usize,
                message: String,
                file: String,
                source: String,
                suggestion: Vec<SlynxSuggestion>,
            ) -> Self {
                Self {
                    ty: $value,
                    line,
                    column_start: column,
                    message,
                    file,
                    source_code: source,
                    suggestion,
                }
            }
        }
    };
}

impl_slynx_error!(new_lexer -> SlynxErrorType::Lexer);
impl_slynx_error!(new_parser -> SlynxErrorType::Parser);
impl_slynx_error!(new_hir -> SlynxErrorType::Hir);
impl_slynx_error!(new_type-> SlynxErrorType::Type);
impl_slynx_error!(new_compiler-> SlynxErrorType::Compilation);
