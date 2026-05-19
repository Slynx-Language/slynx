use color_eyre::owo_colors::OwoColorize;

pub mod checker;
pub mod helpers;
pub mod hir;
pub mod ir;
pub mod lexer;
pub mod parser;

use helpers::SlynxSuggestion;

use crate::helpers::ErrorMetadata;

///A column range that's is represented by 48 bits for indexing, and 16 bits to define its range.
#[derive(Debug)]
pub struct ColumnRange(u64);
impl ColumnRange {
    pub fn new(start: usize, end: usize) -> Self {
        let start = start as u64;
        let end = end as u64;
        let dif = end - start;
        Self(start << 32 | dif & 0xffffffff)
    }

    pub fn start(&self) -> usize {
        (self.0 >> 32) as usize
    }
    pub fn end(&self) -> usize {
        self.start() + (self.0 as usize & 0xffffffff)
    }
}

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
    ///The phase this error was generated at
    pub ty: SlynxErrorType,
    ///The initial line this error occurs
    pub line: usize,
    ///The starting column of this error. Thus, where it initializes inside a buffer
    pub column: ColumnRange,
    ///All the metadata of this error, such as File, Message and Source
    pub metadata: ErrorMetadata,
    ///The file path the error occuried
    ///Suggestions for solving this error
    pub suggestion: Vec<SlynxSuggestion>,
}

impl std::error::Error for SlynxError {}

impl std::fmt::Display for SlynxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_error = self.ty.to_string();
        let source = self.metadata.source().replace("\t", " ");
        let before_error = format!("{} |", self.line);
        let error_with_data = format!("{before_error}{source}");

        let error_points = {
            let points = "^".repeat(self.metadata.source().trim().len());
            format!("{before_error}{points}",)
        };

        let line_and_column = format!(
            "{}:{}",
            self.line.blue().bold(),
            self.column.start().blue().bold()
        );
        writeln!(
            f,
            "{}: {} => {}:{line_and_column}",
            type_error.green().bold(),
            self.metadata.message().red(),
            self.metadata.file().bold()
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
                end_column: usize,
                message: String,
                file: String,
                source: String,
                suggestion: Vec<SlynxSuggestion>,
            ) -> Self {
                Self {
                    ty: $value,
                    line,
                    column: ColumnRange::new(column, end_column),
                    metadata: ErrorMetadata::new(file, message, source),
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
