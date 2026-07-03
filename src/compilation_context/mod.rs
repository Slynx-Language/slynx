mod errors;

use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use common::{FrontendSymbol, SymbolsModule, pool::DedupPool};
use dashmap::DashMap;
use module_loader::{Modules, SourceLoader, SourceProvider};
use slynx_codegen::Codegen;
use slynx_hir::SlynxHir;
use slynx_ir::SlynxIR;
use slynx_lexer::{Lexer, TokenStream};
use slynx_monomorphizer::Monomorphizer;
use slynx_parser::{ASTExpression, ASTStatement, GenericIdentifier, Parser, Program};

pub use crate::compilation_context::errors::*;

#[derive(Debug)]
pub struct CompilationOutput {
    output_path: PathBuf,
    ir: SlynxIR,
}

#[derive(Debug)]
pub struct CompilationStages {
    entry_point: PathBuf,
    ir: SlynxIR,
}

pub struct FilesProvider {
    files: DashMap<Arc<PathBuf>, (String, Vec<usize>)>,
}

impl FilesProvider {
    pub fn new() -> Self {
        Self {
            files: DashMap::new(),
        }
    }
    pub fn insert(&self, path: &Path, content: String) {
        let lines = content
            .chars()
            .enumerate()
            .filter_map(|(idx, c)| if c == '\n' { Some(idx) } else { None })
            .collect::<Vec<_>>();

        self.files.insert(Arc::new(path.into()), (content, lines));
    }
}

impl Deref for FilesProvider {
    type Target = DashMap<Arc<PathBuf>, (String, Vec<usize>)>;
    fn deref(&self) -> &Self::Target {
        &self.files
    }
}

impl SourceProvider<'static> for FilesProvider {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        if let Some(file) = self.get(&Arc::new(path.into())) {
            return Ok(file.0.clone());
        }
        let file = std::fs::read_to_string(path)?;
        self.insert(path, file.clone());
        Ok(file)
    }
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
    pub fn write(&self) -> std::io::Result<()> {
        std::fs::write(&self.output_path, format!("{:#?}", self.ir))?;
        Ok(())
    }
}

impl CompilationStages {
    fn new(entry_point: &Path, ir: SlynxIR) -> Self {
        Self {
            entry_point: entry_point.to_path_buf(),
            ir,
        }
    }

    pub fn ir_text(&self) -> String {
        format!("{:#?}", self.ir)
    }

    pub fn dump_path(&self, extension: &str) -> PathBuf {
        self.entry_point.with_extension(extension)
    }

    pub fn write_ir(&self) -> std::io::Result<()> {
        std::fs::write(self.dump_path("ir"), self.ir_text())?;
        Ok(())
    }

    pub fn into_output(self) -> CompilationOutput {
        CompilationOutput::new(self.entry_point.as_path(), self.ir)
    }
}

pub struct GlobalPools {
    names: SymbolsModule<FrontendSymbol>,
    expressions: DedupPool<ASTExpression>,
    statements: DedupPool<ASTStatement>,
    types: DedupPool<GenericIdentifier>,
}
impl GlobalPools {
    pub fn new() -> Self {
        Self {
            names: SymbolsModule::new(),
            expressions: DedupPool::new(),
            statements: DedupPool::new(),
            types: DedupPool::new(),
        }
    }
}

///Context that will have all the information needed when erroring or retrieving metadata about the code itself during compilation.
///For example, this can be used when erroring to retrieve the correct line where the file errored
pub struct SlynxContext {
    ///The source code of the files and their lines. Maps the name of some to its source code and its lines. Can and is used when importing contents(will be implemented yet)
    files: FilesProvider,

    entry_point: Arc<PathBuf>,
    std: PathBuf,
    pools: GlobalPools,
}

pub struct LineInfo {
    ///The line where the error occuried
    pub line: usize,
    ///The initial column on that line
    pub column_start: usize,
    ///The final column on that line
    pub column_end: usize,
    ///The source that generated that error
    pub src: String,
}

impl SlynxContext {
    pub fn std_dir(std_path: Option<PathBuf>) -> PathBuf {
        if let Some(std) = std_path {
            std
        } else if let Ok(path) = std::env::var("STD_PATH") {
            PathBuf::from(path)
        } else {
            std::env::home_dir()
                .expect("Expected to have home dir")
                .join(".slynx")
                .join("std")
        }
    }

    pub fn new(entry_point: PathBuf, std_path: Option<PathBuf>) -> std::io::Result<Self> {
        let entry_point = Arc::new(entry_point);
        let mut out = Self {
            files: FilesProvider::new(),
            entry_point: entry_point.clone(),
            std: Self::std_dir(std_path),
            pools: GlobalPools::new(),
        };
        out.insert_file(&entry_point)?;
        Ok(out)
    }

    pub fn from_source(src: String, root: &Path) -> Self {
        let entry = Arc::new(root.into());
        let provider = FilesProvider::new();
        provider.insert(root, src);

        Self {
            files: provider,
            entry_point: entry,
            std: Self::std_dir(None),
            pools: GlobalPools::new(),
        }
    }

    ///Gets the source code of the file that will start all the compilation
    pub fn get_entry_point_source(&self) -> String {
        self.files
            .get(&self.entry_point)
            .expect("Entry point should map to a file")
            .0
            .clone()
    }

    ///Inserts the file with provided `path` if it exists.
    pub fn insert_file(&mut self, path: &Path) -> std::io::Result<()> {
        self.files.read(path)?;
        Ok(())
    }

    ///Registers a file that was already loaded by the source loader.
    ///Computes line metadata from the provided source without re-reading from disk.
    pub fn register_loaded_file(&self, path: Arc<PathBuf>, source: String) {
        self.files.insert(&path, source);
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
    pub fn get_line_info(&self, path: &Arc<PathBuf>, index: usize) -> LineInfo {
        let guard = self
            .files
            .get(path)
            .expect("Path should be provided on the context");
        let (source, lines) = guard.value();
        if source.is_empty() {
            return LineInfo {
                line: 1,
                column_start: 1,
                column_end: 1,
                src: "".into(),
            };
        }

        let char_len = source.chars().count();
        let clamped_index = index.min(char_len.saturating_sub(1));
        let line_idx = match lines.binary_search(&clamped_index) {
            Ok(line) | Err(line) => line,
        };
        let line_end_char = if line_idx < lines.len() {
            lines[line_idx]
        } else {
            char_len
        };
        let line_start_char = if line_idx == 0 {
            0
        } else {
            line_end_char.min(lines[line_idx - 1] + 1)
        };

        let start = Self::char_index_to_byte_offset(source, line_start_char);
        let end = Self::char_index_to_byte_offset(source, line_end_char);
        let column = end.min(clamped_index.saturating_sub(line_start_char) + 1);

        LineInfo {
            line: line_idx + 1,
            column_start: column,
            column_end: end,
            src: source[start..end].to_string(),
        }
    }

    ///The name of the file this context is parsing
    pub fn file_name(&self) -> String {
        self.entry_point.to_string_lossy().to_string()
    }

    ///Builds the token stream to be used by the Parser from the source code
    pub fn build_tokens(&self) -> Result<TokenStream, SlynxError> {
        Lexer::tokenize(&self.get_entry_point_source()).map_err(|e| self.handle_lexer_error(e))
    }

    ///Builds the Slynx AST from the given `tokens` stream.
    pub fn build_parser(&self, tokens: TokenStream) -> Result<Program, SlynxError> {
        Parser::new(
            tokens,
            &self.pools.names,
            &self.pools.expressions,
            &self.pools.statements,
            &self.pools.types,
        )
        .parse_declarations()
        .map_err(|e| self.handle_parser_error(&e))
    }

    pub fn load_modules<'a>(&'a mut self) -> Result<Modules<'a>, SlynxError> {
        let loader = SourceLoader::new(
            &self.pools.names,
            &self.pools.statements,
            &self.pools.expressions,
            &self.pools.types,
        );

        let std = self.std.clone();
        let entry = (*self.entry_point).clone();
        let mut on_load = |path: &Path, source: &str| {
            self.register_loaded_file(Arc::new(path.to_path_buf()), source.to_string());
        };
        let modules = { loader.load(entry, std, &mut on_load, &self.files) };
        match modules {
            Ok(modules) => Ok(modules),
            Err(e) => Err(self.handle_source_error(&e)),
        }
    }

    ///Builds the Slynx HIR from the given `ast`. And type checks the HIR. The result hir is already typed. Also returns the types module to be used if needed to get information about the types on the Hir.
    pub fn build_hir<'a>(&self, ast: &'a Modules) -> Result<SlynxHir<'a>, SlynxError> {
        let mut hir = SlynxHir::new(&ast).map_err(|e| self.handle_hir_error(&e.0, &e.1))?;

        self.monomorphize(&mut hir)?;

        Ok(hir)
    }

    ///Monomorphization only changes(by now) the types module.
    pub fn monomorphize(&self, hir: &mut SlynxHir) -> Result<(), SlynxError> {
        Monomorphizer::resolve(hir).map_err(|e| self.handle_hir_error(hir, &e))
    }

    ///Builds a new IR from the given `hir`. It's assumed that it is already implemented
    pub fn build_ir(&self, hir: SlynxHir) -> Result<SlynxIR, SlynxError> {
        let mut codegen = Codegen::new();
        codegen
            .generate(&hir)
            .map_err(|e| self.build_ir_generation_error(&e, &hir))
    }

    ///Builds typed HIR and IR once so callers can inspect or persist intermediate dumps
    ///before materializing the default `.sir` output.
    pub fn build_stages(self) -> Result<CompilationStages, SlynxError> {
        let entry = (*self.entry_point).clone();
        let modules = {
            let std = self.std.clone();
            let on_load = |path: &Path, source: &str| {
                self.register_loaded_file(Arc::new(path.to_path_buf()), source.to_string());
            };
            let source = SourceLoader::new(
                &self.pools.names,
                &self.pools.statements,
                &self.pools.expressions,
                &self.pools.types,
            );
            source.load(entry, std, on_load, &self.files)
        };
        let modules = match modules {
            Ok(modules) => modules,
            Err(e) => return Err(self.handle_source_error(&e)),
        };
        let hir = self.build_hir(&modules)?;
        let ir = self.build_ir(hir)?;

        Ok(CompilationStages::new(self.entry_point.as_ref(), ir))
    }

    ///Compiles the code from the current contexts and returns the compilation result including the IR
    pub fn compile(self) -> Result<CompilationOutput, SlynxError> {
        let stages = self.build_stages()?;
        Ok(stages.into_output())
    }
}

#[cfg(test)]
mod tests {
    use super::SlynxContext;

    use std::{
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
            SlynxContext::new((*path).clone(), None).expect("context should be created"),
            path,
            dir,
        )
    }

    #[test]
    fn get_line_info_handles_single_line_sources_without_trailing_newline() {
        let (context, path, dir) = temp_context("func main(): int {", "single-line");

        let info = context.get_line_info(&path, 5);

        assert_eq!(info.line, 1);
        assert_eq!(info.column_start, 6);
        assert_eq!(info.src, "func main(): int {");

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }

    #[test]
    fn get_line_info_handles_last_line_without_trailing_newline() {
        let source = "func main(): int {\n    let value = 1;\n    value";
        let (context, path, dir) = temp_context(source, "last-line");

        let last_line_start = source.rfind('\n').expect("last line should exist") + 1;
        let value_index = source[..last_line_start].chars().count() + 4;
        let info = context.get_line_info(&path, value_index);

        assert_eq!(info.line, 3);
        assert_eq!(info.column_start, 5);
        assert_eq!(info.src, "    value");

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }

    #[test]
    fn get_line_info_supports_non_ascii_columns_without_panicking() {
        let source = "a\u{00E7}\u{00E3}o\n\u{03B2}";
        let (context, path, dir) = temp_context(source, "utf8");

        let info = context.get_line_info(&path, 2);

        assert_eq!(info.line, 1);
        assert_eq!(info.column_start, 3);
        assert_eq!(info.src, "a\u{00E7}\u{00E3}o");

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }

    #[test]
    fn get_line_info_handles_empty_sources() {
        let (context, path, dir) = temp_context("", "empty");

        let info = context.get_line_info(&path, 0);

        assert_eq!(info.line, 1);
        assert_eq!(info.column_start, 1);
        assert_eq!(info.src, "");

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }
}
