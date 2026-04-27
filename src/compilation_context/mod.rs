mod errors;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use color_eyre::{eyre::Result, owo_colors::OwoColorize};

use common::SymbolPointer;
use frontend::{
    hir::{
        SlynxHir, VariableId,
        modules::{DeclarationsModule, TypesModule},
    },
    lexer::Lexer,
    parser::Parser,
};

use frontend::{
    checker::{TypeChecker, error::TypeError},
    monomorphizer::Monomorphizer,
};
use middleend::{IRError, SlynxIR};

use crate::compilation_context::errors::helpers::{
    SlynxSuggestion, suggestions_from_lexer, suggestions_from_type_error,
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

#[derive(Debug)]
pub struct CompilationStages {
    entry_point: PathBuf,
    hir_dump: String,
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
        self.output_path.as_path()
    }

    ///Writes the IR of this output into the path of `output_path()`
    pub fn write(&self) -> Result<()> {
        std::fs::write(&self.output_path, format!("{:#?}", self.ir))?;
        Ok(())
    }
}

impl CompilationStages {
    fn new(entry_point: &Path, hir_dump: String, ir: SlynxIR) -> Self {
        Self {
            entry_point: entry_point.to_path_buf(),
            hir_dump,
            ir,
        }
    }

    pub fn hir_text(&self) -> &str {
        &self.hir_dump
    }

    pub fn ir_text(&self) -> String {
        format!("{:#?}", self.ir)
    }

    pub fn dump_path(&self, extension: &str) -> PathBuf {
        self.entry_point.with_extension(extension)
    }

    pub fn write_hir(&self) -> Result<()> {
        std::fs::write(self.dump_path("hir"), self.hir_text())?;
        Ok(())
    }

    pub fn write_ir(&self) -> Result<()> {
        std::fs::write(self.dump_path("ir"), self.ir_text())?;
        Ok(())
    }

    pub fn into_output(self) -> CompilationOutput {
        CompilationOutput::new(self.entry_point.as_path(), self.ir)
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

    fn char_index_to_byte_offset(source: &str, char_index: usize) -> usize {
        if char_index == 0 {
            return 0;
        }

        source
            .char_indices()
            .nth(char_index)
            .map(|(offset, _)| offset)
            .unwrap_or(source.len())
    }

    fn entry_point_eof_index(&self) -> usize {
        self.get_entry_point_source()
            .chars()
            .count()
            .saturating_sub(1)
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
        if source.is_empty() {
            return (1, 1, "");
        }

        let char_len = source.chars().count();
        let clamped_index = index.min(char_len.saturating_sub(1));
        let line_idx = match lines.binary_search(&clamped_index) {
            Ok(line) | Err(line) => line,
        };

        let line_start_char = if line_idx == 0 {
            0
        } else {
            lines[line_idx - 1] + 1
        };
        let line_end_char = if line_idx < lines.len() {
            lines[line_idx]
        } else {
            char_len
        };

        let start = Self::char_index_to_byte_offset(source, line_start_char);
        let end = Self::char_index_to_byte_offset(source, line_end_char);
        let column = clamped_index.saturating_sub(line_start_char) + 1;

        (line_idx + 1, column, &source[start..end])
    }

    ///The name of the file this context is parsing
    pub fn file_name(&self) -> String {
        self.entry_point.to_string_lossy().to_string()
    }

    ///Builds typed HIR and IR once so callers can inspect or persist intermediate dumps
    ///before materializing the default `.sir` output.
    pub fn build_stages(self) -> Result<CompilationStages> {
        let stream = match Lexer::tokenize(self.get_entry_point_source()) {
            Ok(value) => value,
            Err(e) => return Err(self.handle_lexer_error(e)),
        };
        let decls = match Parser::new(stream).parse_declarations() {
            Ok(v) => v,
            Err(e) => return Err(self.handle_parser_error(e.downcast_ref().unwrap()).into()),
        };
        let mut hir = SlynxHir::new();
        if let Err(e) = hir.generate(decls) {
            return Err(self.handle_hir_error(&hir, &e));
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
            return Err(self.handle_hir_error(&hir, &e));
        }
        let hir_dump = format_hir_dump(&hir, &types_module);
        let variable_names = hir.modules.symbols_resolver.variables().clone();
        let mut ir = SlynxIR::new(hir.modules.symbols_resolver.get_symbols_module());

        if let Err(e) = ir.generate(hir.declarations, &types_module) {
            return Err(self
                .build_ir_generation_error(
                    &e,
                    &ir,
                    &variable_names,
                    &types_module,
                    &hir.modules.declarations_module,
                )
                .into());
        };
        Ok(CompilationStages::new(
            self.entry_point.as_ref(),
            hir_dump,
            ir,
        ))
    }

    ///Compiles the code from the current contexts and returns the compilation result including the IR
    pub fn compile(self) -> Result<CompilationOutput> {
        let stages = self.build_stages()?;
        Ok(stages.into_output())
    }

    pub fn start_compilation(self) -> Result<()> {
        let output = self.compile()?;
        output.write()?;
        Ok(())
    }
}

fn format_hir_dump(hir: &SlynxHir, types_module: &TypesModule) -> String {
    format!(
        "HIR Declarations:\n{:#?}\n\nDeclarations Module:\n{:#?}\n\nTypes Module:\n{:#?}\n\nVariable Names:\n{:#?}",
        hir.declarations,
        hir.modules.types_module,
        types_module,
        hir.modules.symbols_resolver.variables()
    )
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
    use super::SlynxContext;
    use super::format_ir_generation_error;
    use frontend::hir::modules::BUILTIN_NAMES;
    use frontend::hir::{
        DeclarationId, VariableId,
        model::HirType,
        modules::{DeclarationsModule, SymbolsModule, TypesModule},
    };
    use middleend::{IRError, SlynxIR};
    use std::{
        collections::HashMap,
        fs,
        path::PathBuf,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_context(source: &str, name: &str) -> (SlynxContext, Arc<PathBuf>, PathBuf) {
        let mut dir = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        dir.push(format!(
            "slynx-context-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp dir should be created");

        let path = Arc::new(dir.join("input.slynx"));
        fs::write(path.as_ref(), source).expect("temp source should be written");

        (
            SlynxContext::new(path.clone()).expect("context should be created"),
            path,
            dir,
        )
    }

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

    #[test]
    fn get_line_info_handles_single_line_sources_without_trailing_newline() {
        let (context, path, dir) = temp_context("func main(): int {", "single-line");

        let (line, column, source) = context.get_line_info(&path, 5);

        assert_eq!(line, 1);
        assert_eq!(column, 6);
        assert_eq!(source, "func main(): int {");

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }

    #[test]
    fn get_line_info_handles_last_line_without_trailing_newline() {
        let source = "func main(): int {\n    let value = 1;\n    value";
        let (context, path, dir) = temp_context(source, "last-line");

        let last_line_start = source.rfind('\n').expect("last line should exist") + 1;
        let value_index = source[..last_line_start].chars().count() + 4;
        let (line, column, line_source) = context.get_line_info(&path, value_index);

        assert_eq!(line, 3);
        assert_eq!(column, 5);
        assert_eq!(line_source, "    value");

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }

    #[test]
    fn get_line_info_supports_non_ascii_columns_without_panicking() {
        let source = "a\u{00E7}\u{00E3}o\n\u{03B2}";
        let (context, path, dir) = temp_context(source, "utf8");

        let (line, column, line_source) = context.get_line_info(&path, 2);

        assert_eq!(line, 1);
        assert_eq!(column, 3);
        assert_eq!(line_source, "a\u{00E7}\u{00E3}o");

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }

    #[test]
    fn get_line_info_handles_empty_sources() {
        let (context, path, dir) = temp_context("", "empty");

        let (line, column, line_source) = context.get_line_info(&path, 0);

        assert_eq!(line, 1);
        assert_eq!(column, 1);
        assert_eq!(line_source, "");

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }
}
