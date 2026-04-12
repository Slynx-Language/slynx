use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use color_eyre::{Report, eyre::Result, owo_colors::OwoColorize};

use frontend::hir::{
    SlynxHir, VariableId, declarations::DeclarationsModule, error::HIRError,
    symbols::SymbolPointer, types::TypesModule,
};
use frontend::lexer::{Lexer, error::LexerError};
use frontend::parser::{Parser, error::ParseError};
use frontend::{
    checker::{TypeChecker, error::TypeError},
    monomorphizer::Monomorphizer,
};
use middleend::{IRError, SlynxIR};

use crate::err::{
    SlynxSuggestion, suggestions_from_hir, suggestions_from_ir, suggestions_from_lexer,
    suggestions_from_parser, suggestions_from_type_error,
};
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
        writeln!(f, "{}", error_points)?;
        for suggestion in self.suggestion.iter() {
            writeln!(f, "-->{}", suggestion)?;
        }
        writeln!(f)
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

    fn build_ir_generation_error(
        &self,
        error: &IRError,
        ir: &SlynxIR,
        variable_names: &HashMap<VariableId, SymbolPointer>,
        types_module: &TypesModule,
        declarations_module: &DeclarationsModule,
    ) -> SlynxError {
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
    }

    ///Compiles the code from the current contexts and returns the compilation result including the IR
    pub fn compile(self) -> Result<CompilationOutput> {
        let stream = match Lexer::tokenize(self.get_entry_point_source()) {
            Ok(value) => value,
            Err(e) => {
                let suggestion = suggestions_from_lexer(&e);
                match e {
                    LexerError::MalformedNumber { init, .. } => {
                        let (line, column, src) = self.get_line_info(&self.entry_point, init);
                        let err = SlynxError {
                            line,
                            ty: SlynxErrorType::Lexer,
                            column_start: column,
                            message: e.to_string(),
                            suggestion,
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
                            suggestion,
                            file: self.entry_point.to_string_lossy().to_string(),
                            source_code: src.to_string(),
                        };
                        return Err(Report::new(err));
                    }
                }
            }
        };
        let decls = match Parser::new(stream).parse_declarations() {
            Ok(v) => v,
            Err(e) => {
                return match e.downcast_ref::<ParseError>() {
                    Some(err @ ParseError::UnexpectedToken(token, _)) => {
                        let (line, column, src) =
                            self.get_line_info(&self.entry_point, token.span.start);
                        let suggestion = suggestions_from_parser(err);
                        let err = SlynxError {
                            line,
                            ty: SlynxErrorType::Parser,
                            column_start: column,
                            message: err.to_string(),
                            file: self.file_name(),
                            suggestion,
                            source_code: src.to_string(),
                        };
                        Err(e.wrap_err(err))
                    }
                    Some(err @ ParseError::UnexpectedEndOfInput) => {
                        let suggestion = suggestions_from_parser(err);
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
                            suggestion,
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
                    let suggestion = suggestions_from_hir(err);
                    let (line, column, src) = self.get_line_info(&self.entry_point, err.span.start);
                    let err = SlynxError {
                        line,
                        column_start: column,
                        ty: SlynxErrorType::Hir,
                        message: e.to_string(),
                        suggestion,
                        file: self.entry_point.to_string_lossy().to_string(),
                        source_code: src.to_string(),
                    };
                    return Err(e.wrap_err(err));
                }
                None => return Err(e),
            }
        }
        let mut types_module = match TypeChecker::check(&mut hir) {
            Err(e) => match e.downcast_ref::<TypeError>() {
                Some(err) => {
                    let suggestion = suggestions_from_type_error(err);
                    let (line, column, src) = self.get_line_info(&self.entry_point, err.span.start);
                    let err = SlynxError {
                        line,
                        column_start: column,
                        suggestion,
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
        if let Err(e) = Monomorphizer::resolve(&hir, &mut types_module) {
            match e.downcast_ref::<HIRError>() {
                Some(err) => {
                    let suggestion = suggestions_from_hir(err);
                    let (line, column, src) = self.get_line_info(&self.entry_point, err.span.start);
                    let err = SlynxError {
                        line,
                        column_start: column,
                        ty: SlynxErrorType::Hir,
                        message: e.to_string(),
                        suggestion,
                        file: self.entry_point.to_string_lossy().to_string(),
                        source_code: src.to_string(),
                    };
                    return Err(e.wrap_err(err));
                }
                None => return Err(e),
            }
        }
        let variable_names = hir.variable_names().clone();
        let mut ir = SlynxIR::new(hir.symbols_module);

        if let Err(e) = ir.generate(hir.declarations, &types_module) {
            return Err(self
                .build_ir_generation_error(
                    &e,
                    &ir,
                    &variable_names,
                    &types_module,
                    &hir.declarations_module,
                )
                .into());
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

fn format_ir_generation_error(
    error: &IRError,
    ir: &SlynxIR,
    variable_names: &HashMap<VariableId, SymbolPointer>,
    types_module: &TypesModule,
    declarations_module: &DeclarationsModule,
) -> String {
    match error {
        IRError::UnrecognizedVariable(id) => {
            if let Some(name) = variable_names
                .get(id)
                .copied()
                .map(|symbol| ir.string_pool().get_name(symbol))
            {
                format!("IR internal error: variable '{name}' is not recognized by the IR")
            } else {
                format!(
                    "IR internal error: variable id {} is not recognized by the IR",
                    id.as_raw()
                )
            }
        }
        IRError::DeclarationNotRecognized(id) => {
            if let Some(name) = declarations_module
                .try_retrieve_declaration_type(*id)
                .and_then(|ty| types_module.get_type_name(&ty).copied())
                .map(|symbol| ir.string_pool().get_name(symbol))
            {
                format!("IR internal error: declaration '{name}' is not recognized by the IR")
            } else {
                format!(
                    "IR internal error: declaration id {} is not recognized by the IR",
                    id.as_raw()
                )
            }
        }
        IRError::IRTypeNotRecognized(id) => {
            if let Some(name) = types_module
                .get_type_name(id)
                .copied()
                .map(|symbol| ir.string_pool().get_name(symbol))
            {
                format!("IR internal error: type '{name}' is not recognized by the IR")
            } else {
                format!(
                    "IR internal error: type id {} is not recognized by the IR",
                    id.as_raw()
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_ir_generation_error;
    use frontend::hir::{
        DeclarationId, VariableId,
        declarations::DeclarationsModule,
        symbols::SymbolsModule,
        types::{BUILTIN_NAMES, HirType, TypesModule},
    };
    use middleend::{IRError, SlynxIR};
    use std::collections::HashMap;

    #[test]
    fn formats_variable_ir_errors_with_source_names() {
        let mut symbols = SymbolsModule::new();
        let builtins = BUILTIN_NAMES.map(|name| symbols.intern(name));
        let variable_name = symbols.intern("count");
        let mut types = TypesModule::new(&builtins);
        let mut variable_names = HashMap::new();
        let declarations = DeclarationsModule::new();
        let ir = SlynxIR::new(symbols);

        let variable = VariableId::from_raw(77);
        types.insert_variable(variable, types.int_id());
        variable_names.insert(variable, variable_name);

        assert_eq!(
            format_ir_generation_error(
                &IRError::UnrecognizedVariable(variable),
                &ir,
                &variable_names,
                &types,
                &declarations
            ),
            "IR internal error: variable 'count' is not recognized by the IR"
        );
    }

    #[test]
    fn formats_declaration_ir_errors_with_source_names() {
        let mut symbols = SymbolsModule::new();
        let builtins = BUILTIN_NAMES.map(|name| symbols.intern(name));
        let declaration_name = symbols.intern("Bordered");
        let mut types = TypesModule::new(&builtins);
        let variable_names = HashMap::new();
        let mut declarations = DeclarationsModule::new();
        let ir = SlynxIR::new(symbols);

        let ty = types.insert_type(declaration_name, HirType::Component { props: Vec::new() });
        let declaration = declarations.create_declaration(declaration_name, ty);

        assert_eq!(
            format_ir_generation_error(
                &IRError::DeclarationNotRecognized(declaration),
                &ir,
                &variable_names,
                &types,
                &declarations
            ),
            "IR internal error: declaration 'Bordered' is not recognized by the IR"
        );
    }

    #[test]
    fn formats_type_ir_errors_with_source_names() {
        let mut symbols = SymbolsModule::new();
        let builtins = BUILTIN_NAMES.map(|name| symbols.intern(name));
        let type_name = symbols.intern("User");
        let mut types = TypesModule::new(&builtins);
        let variable_names = HashMap::new();
        let declarations = DeclarationsModule::new();
        let ir = SlynxIR::new(symbols);

        let ty = types.insert_type(type_name, HirType::Struct { fields: Vec::new() });

        assert_eq!(
            format_ir_generation_error(
                &IRError::IRTypeNotRecognized(ty),
                &ir,
                &variable_names,
                &types,
                &declarations
            ),
            "IR internal error: type 'User' is not recognized by the IR"
        );
    }

    #[test]
    fn falls_back_to_ids_when_names_are_missing() {
        let mut symbols = SymbolsModule::new();
        let builtins = BUILTIN_NAMES.map(|name| symbols.intern(name));
        let types = TypesModule::new(&builtins);
        let variable_names = HashMap::new();
        let declarations = DeclarationsModule::new();
        let ir = SlynxIR::new(symbols);

        assert_eq!(
            format_ir_generation_error(
                &IRError::UnrecognizedVariable(VariableId::from_raw(5)),
                &ir,
                &variable_names,
                &types,
                &declarations
            ),
            "IR internal error: variable id 5 is not recognized by the IR"
        );
        assert_eq!(
            format_ir_generation_error(
                &IRError::DeclarationNotRecognized(DeclarationId::from_raw(9)),
                &ir,
                &variable_names,
                &types,
                &declarations
            ),
            "IR internal error: declaration id 9 is not recognized by the IR"
        );
    }
}
