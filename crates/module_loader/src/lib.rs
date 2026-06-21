mod file;
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
};

use common::Span;
use slynx_lexer::{Lexer, LexerError};
use slynx_parser::{ASTDeclaration, ASTDeclarationKind, ASTPath, ParseError, Parser};

pub struct Modules {}

#[derive(Debug)]
///A module that was parsed
pub struct SourceNode {
    ///The path to the file that generated this module
    pub id: FileId,
    ///The declarations inside this module
    pub declarations: Vec<ASTDeclaration>,
    ///Per-import submodule lists: `import_submodules[i]` contains the FileIds for the i-th import declaration.
    pub import_submodules: Vec<Vec<FileId>>,
}

#[derive(Debug)]
pub enum SourceErrorKind {
    InexsitantSource(std::io::Error, Span, PathBuf),
    Lexing(LexerError),
    Parsing(ParseError),
}

#[derive(Debug)]
pub struct SourceError {
    kind: SourceErrorKind,
    entry: PathBuf,
}

impl SourceError {
    ///Creates a new `inexistant_module` error with the given `e` io error, `entry` entry point that tried to be read, `generator` being the path of the file that generated that error and `span` to emit where it happened
    pub fn inexistant_module(
        e: std::io::Error,
        entry: PathBuf,
        generator: PathBuf,
        span: Span,
    ) -> Self {
        Self {
            kind: SourceErrorKind::InexsitantSource(e, span, generator),
            entry: entry.clone(),
        }
    }
    pub fn lexing(e: LexerError, entry: PathBuf) -> Self {
        Self {
            kind: SourceErrorKind::Lexing(e),
            entry: entry.clone(),
        }
    }
    pub fn parsing(e: ParseError, entry: PathBuf) -> Self {
        Self {
            kind: SourceErrorKind::Parsing(e),
            entry: entry.clone(),
        }
    }
    pub fn kind(&self) -> &SourceErrorKind {
        &self.kind
    }
    pub fn entry(&self) -> &Path {
        &self.entry
    }
}

impl SourceNode {
    pub fn new(id: FileId, declarations: Vec<ASTDeclaration>) -> Self {
        Self {
            id,
            declarations,
            import_submodules: Vec::new(),
        }
    }
}

pub struct SourceLoader;

pub struct SourceInfo {
    module: SourceNode,
    /// Per-import pending paths: pending[i] are the paths for the i-th import declaration.
    pending: Vec<Vec<PathBuf>>,
}

impl SourceLoader {
    ///Resolves the given `entry` path with the given module names. Returns the resultant path, and if it's a folder or a file. True for folders, false for File
    fn resolve_ast_path(
        module_names: &[String],
        entry: &Path,
    ) -> Result<(PathBuf, bool), std::io::Error> {
        let mut entry = entry.to_path_buf();
        let mut folder = module_names.last().is_none();
        if let Some((last, module_names)) = module_names.split_last() {
            for name in module_names {
                entry.push(name);
            }
            entry.push(last);

            folder = entry
                .metadata()
                .map(|metadata| metadata.is_dir())
                .unwrap_or(false);
            if !folder {
                entry.set_extension("slx");
            };
        }
        Ok((entry, folder))
    }

    fn resolve_path(
        path: &ASTPath,
        global_entry: &Path,
        current: &Path,
        std_path: &Path,
    ) -> Result<(PathBuf, bool), std::io::Error> {
        let path = &path.module_names;
        match path[0].as_str() {
            "root" => Self::resolve_ast_path(&path[1..], global_entry),
            "std" => Self::resolve_ast_path(&path[1..], std_path),
            _ => Self::resolve_ast_path(path, current),
        }
    }

    fn load_file_module(
        mut entry: PathBuf,
        global_entry: &Path,
        std_path: &Path,
        id: FileId,
        importer: &Path,
        on_file_loaded: &mut impl FnMut(&Path, &str),
    ) -> Result<SourceInfo, SourceError> {
        let generator = entry.clone();
        let source = std::fs::read_to_string(&entry).map_err(|e| {
            SourceError::inexistant_module(
                e,
                entry.clone(),
                importer.to_path_buf(),
                Span { start: 0, end: 0 },
            )
        })?;
        on_file_loaded(&entry, &source);
        let tokens = Lexer::tokenize(&source).map_err(|e| SourceError::lexing(e, entry.clone()))?;
        let declarations = Parser::new(tokens)
            .parse_declarations()
            .map_err(|e| SourceError::parsing(e, entry.clone()))?;

        entry.pop(); //since its a file, we need to track its current folder to be able to get the siblings, and so we pop the name
        let mut pending: Vec<Vec<PathBuf>> = Vec::new();
        for decl in &declarations {
            if let ASTDeclarationKind::Import(ref import) = decl.kind {
                let (resolved, is_folder) = Self::resolve_path(
                    &import.path,
                    global_entry,
                    &entry,
                    std_path,
                )
                .map_err(|e| {
                    SourceError::inexistant_module(e, entry.clone(), generator.clone(), decl.span)
                })?;
                let mut import_paths = Vec::new();
                if is_folder {
                    for dir_entry in std::fs::read_dir(&resolved).map_err(|e| {
                        SourceError::inexistant_module(
                            e,
                            resolved.clone(),
                            generator.clone(),
                            decl.span,
                        )
                    })? {
                        let path = dir_entry
                            .map_err(|e| {
                                SourceError::inexistant_module(
                                    e,
                                    resolved.clone(),
                                    generator.clone(),
                                    decl.span,
                                )
                            })?
                            .path();
                        import_paths.push(path);
                    }
                } else {
                    import_paths.push(resolved);
                }
                pending.push(import_paths);
            }
        }
        Ok(SourceInfo {
            module: SourceNode {
                id,
                declarations,
                import_submodules: Vec::new(),
            },
            pending,
        })
    }

    pub fn load(
        entry: PathBuf,
        on_file_loaded: &mut impl FnMut(&Path, &str),
        std_path: PathBuf,
    ) -> Result<Vec<SourceNode>, SourceError> {
        let mut global_entry = entry.clone();
        global_entry.pop(); //pops cause the entry should be a file
        // (path, parent_module_index, import_index_within_parent)
        let mut pending_entries: VecDeque<(PathBuf, Option<(usize, usize)>)> = VecDeque::new();
        pending_entries.push_back((entry, None));
        let mut modules: Vec<SourceNode> = Vec::new();
        let mut module_paths: Vec<PathBuf> = Vec::new(); // parallel to modules
        let mut last_id = FileId(0);
        let mut entry_index = 0;
        let mut path_to_id: HashMap<PathBuf, FileId> = HashMap::new();
        while let Some((entry, parent)) = pending_entries.pop_front() {
            let importer = match parent {
                Some((mod_idx, _)) => module_paths[mod_idx].clone(),
                None => entry.clone(),
            };
            let canonical = std::fs::canonicalize(&entry).unwrap_or_else(|_| entry.clone());
            if let Some(&existing_id) = path_to_id.get(&canonical) {
                if let Some((mod_idx, imp_idx)) = parent {
                    modules[mod_idx].import_submodules[imp_idx].push(existing_id);
                }
                continue;
            }
            path_to_id.insert(canonical, last_id);
            let module_path = entry.clone();
            let module = Self::load_file_module(
                entry,
                &global_entry,
                &std_path,
                last_id,
                &importer,
                on_file_loaded,
            )?;
            module_paths.push(module_path);
            // Pre-allocate per-import slot for this new module's imports
            let import_count = module.pending.len();
            for (imp_idx, import_paths) in module.pending.into_iter().enumerate() {
                for path in import_paths {
                    pending_entries.push_back((path, Some((entry_index, imp_idx))));
                }
            }
            let mut node = module.module;
            node.import_submodules = vec![Vec::new(); import_count];
            if let Some((mod_idx, imp_idx)) = parent {
                modules[mod_idx].import_submodules[imp_idx].push(last_id);
            }
            modules.push(node);
            last_id = last_id.next();
            entry_index += 1;
        }

        Ok(modules)
    }
}
