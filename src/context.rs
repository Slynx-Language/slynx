use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use color_eyre::{Report, eyre::Result, owo_colors::OwoColorize};

use frontend::checker::{TypeChecker, error::TypeError};
use frontend::hir::{SlynxHir, error::HIRError};
use frontend::lexer::{Lexer, error::LexerError};
use frontend::parser::{Parser, error::ParseError};
use middleend::SlynxIR;

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
}
impl std::error::Error for SlynxError {}

#[derive(Debug)]
pub struct CompilationOutput {
    output_path: PathBuf,
    ir: SlynxIR,
}

impl CompilationOutput {
    ///Creates a new compilation output with the provided `ir`. Writes the `ir` in its textual format on the provided `entry_point` with extension of `sir`
    fn new(entry_point: &Path, ir: SlynxIR) -> Self {
        Self {
            output_path: entry_point.with_extension("sir"),
            ir,
        }
    }

    ///Consumes and retrieves the IR of this compilation output
    pub fn ir(self) -> SlynxIR {
        self.ir
    }

    ///Retrieves the path where this compilation output should write the IR at
    pub fn output_path(&self) -> &Path {
        &self.output_path
    }

    ///Writes the IR of this output into the path of `output_path()`
    pub fn write(&self) -> Result<()> {
        std::fs::write(&self.output_path, format!("{:#?}", self.ir))?;
        Ok(())
    }
}

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
        writeln!(f, "{}", error_points)
    }
}

///Context that will have all the information needed when erroring or retrieving metadata about the code itself during compilation.
///For example, this can be used when erroring to retrieve the correct line where the file errored
pub struct SlynxContext {
    ///The source code of the files. Maps the name of some to it's source code. Can and is used when importing contents(will be implemented yet)
    files: HashMap<Arc<PathBuf>, String>,
    ///Maps the name of some file to it's lines. Used when wanting to retrieve for example, returning the lines where an error occuried
    lines: HashMap<Arc<PathBuf>, Vec<usize>>,
    entry_point: Arc<PathBuf>,
}

impl SlynxContext {
    pub fn new(entry_point: Arc<PathBuf>) -> Result<Self> {
        let mut out = Self {
            files: HashMap::new(),
            lines: HashMap::new(),
            entry_point: entry_point.clone(),
        };
        out.insert_file(entry_point)?;
        Ok(out)
    }

    ///Gets the source code of the file that will start all the compilation
    pub fn get_entry_point_source(&self) -> &str {
        self.files
            .get(&self.entry_point)
            .expect("Entry point should map to a file")
    }

    ///Inserts the file with provided `path` if it exists.
    pub fn insert_file(&mut self, path: Arc<PathBuf>) -> Result<()> {
        let file = std::fs::read_to_string(path.as_path())?;
        let lines = file
            .chars()
            .enumerate()
            .filter_map(|(idx, c)| if c == '\n' { Some(idx) } else { None })
            .collect::<Vec<_>>();

        self.files.insert(path.clone(), file);
        self.lines.insert(path, lines);
        Ok(())
    }

    ///Based on the provided `index`, which is the index of a char on the source code of `path`, returns the line where it's located on the file of the provided `path`.
    ///This will return its line and the column and the line containing the error
    pub fn get_line_info(&self, path: &Arc<PathBuf>, index: usize) -> (usize, usize, &str) {
        let lines = self
            .lines
            .get(path)
            .expect("Path should be provided on the context");
        let source = self
            .files
            .get(path)
            .expect("Path should be provided on the context");
        match lines.binary_search(&index) {
            Ok(v) => (
                v,
                index - lines[v],
                &source[lines[index - lines[v]]..lines[v + 1]],
            ),
            Err(e) => (
                e + 1,
                {
                    let mut column = index.saturating_sub(lines[e.saturating_sub(1)]);
                    if column == 0 {
                        column = index + 1;
                    }
                    column
                },
                if e == 0 {
                    &source[0..lines[e]]
                } else {
                    &source[lines[e - 1] + 1..lines[e]]
                },
            ),
        }
    }

    ///The name of the file this context is parsing
    pub fn file_name(&self) -> String {
        self.entry_point.to_string_lossy().to_string()
    }

    ///Compiles the code from the current contexts and returns the compilation result including the IR
    pub fn compile(self) -> Result<CompilationOutput> {
        let stream = match Lexer::tokenize(self.get_entry_point_source()) {
            Ok(value) => value,
            Err(e) => match e {
                LexerError::MalformedNumber { init, .. } => {
                    let (line, column, src) = self.get_line_info(&self.entry_point, init);
                    let err = SlynxError {
                        line,
                        ty: SlynxErrorType::Lexer,
                        column_start: column,
                        message: e.to_string(),
                        file: self.entry_point.to_string_lossy().to_string(),
                        source_code: src.to_string(),
                    };
                    return Err(Report::new(err));
                }
                LexerError::UnrecognizedChar { index, .. } => {
                    let (line, column, src) = self.get_line_info(&self.entry_point, index);
                    let err = SlynxError {
                        line,
                        ty: SlynxErrorType::Lexer,
                        column_start: column,
                        message: e.to_string(),
                        file: self.entry_point.to_string_lossy().to_string(),
                        source_code: src.to_string(),
                    };
                    return Err(Report::new(err));
                }
            },
        };
        let decls = match Parser::new(stream).parse_declarations() {
            Ok(v) => v,
            Err(e) => {
                return match e.downcast_ref::<ParseError>() {
                    Some(ref err @ ParseError::UnexpectedToken(token, _)) => {
                        let (line, column, src) =
                            self.get_line_info(&self.entry_point, token.span.start);
                        let err = SlynxError {
                            line,
                            ty: SlynxErrorType::Parser,
                            column_start: column,
                            message: err.to_string(),
                            file: self.file_name(),
                            source_code: src.to_string(),
                        };
                        Err(e.wrap_err(err))
                    }
                    Some(ParseError::UnexpectedEndOfInput) => {
                        let (line, column, src) = self.get_line_info(
                            &self.entry_point,
                            self.lines.get(&self.entry_point).unwrap().len().max(1) - 1,
                        );
                        let err = SlynxError {
                            line,
                            ty: SlynxErrorType::Parser,
                            column_start: column,
                            message: e.to_string(),
                            file: self.file_name(),
                            source_code: src.to_string(),
                        };
                        Err(e.wrap_err(err))
                    }
                    None => Err(e),
                };
            }
        };
        let mut hir = SlynxHir::new();
        if let Err(e) = hir.generate(decls) {
            match e.downcast_ref::<HIRError>() {
                Some(err) => {
                    let (line, column, src) = self.get_line_info(&self.entry_point, err.span.start);
                    let err = SlynxError {
                        line,
                        column_start: column,
                        ty: SlynxErrorType::Hir,
                        message: e.to_string(),
                        file: self.entry_point.to_string_lossy().to_string(),
                        source_code: src.to_string(),
                    };
                    return Err(e.wrap_err(err));
                }
                None => return Err(e),
            }
        }
        let module = match TypeChecker::check(&mut hir) {
            Err(e) => match e.downcast_ref::<TypeError>() {
                Some(err) => {
                    let (line, column, src) = self.get_line_info(&self.entry_point, err.span.start);
                    let err = SlynxError {
                        line,
                        column_start: column,
                        ty: SlynxErrorType::Type,
                        message: e.to_string(),
                        file: self.file_name(),
                        source_code: src.to_string(),
                    };
                    return Err(e.wrap_err(err));
                }
                _ => return Err(e),
            },
            Ok(module) => module,
        };
        let mut ir = SlynxIR::new();
        let symbols = std::mem::take(&mut hir.symbols_module);
        if let Err(e) = ir.generate(hir.declarations, module, symbols) {
            return Err(color_eyre::eyre::eyre!(format!("IR Generation Error: {:?}", e)));
        };
        let output = CompilationOutput::new(self.entry_point.as_ref(), ir);
        Ok(output)
    }

    pub fn start_compilation(self) -> Result<()> {
        let output = self.compile()?;
        output.write()?;
        Ok(())
    }
}
