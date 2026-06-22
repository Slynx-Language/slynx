mod modules;
pub use modules::*;
mod sources;
pub use sources::*;
mod error;
pub use error::*;
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
};

use common::{FrontendSymbol, Span, SymbolsModule, pool::Pool};
use slynx_lexer::Lexer;
use slynx_parser::{ASTExpression, ASTPath, ASTStatement, FileImport, GenericIdentifier, Parser};
use slynx_parser::{Program, SymbolPointer};

pub struct SourceLoader {
    symbols: SymbolsModule<FrontendSymbol>,
    statements: Pool<ASTStatement>,
    expressions: Pool<ASTExpression>,
    types: Pool<GenericIdentifier>,
}

impl SourceLoader {
    pub fn new() -> Self {
        Self {
            symbols: SymbolsModule::<FrontendSymbol>::new(),
            statements: Pool::new(),
            expressions: Pool::new(),
            types: Pool::new(),
        }
    }

    ///Generates the 'Program' of the given `entry`
    fn read_file(
        &self,
        entry: &Path,
        importer: &Path,
        on_file_loaded: &mut impl FnMut(&Path, &str),
    ) -> Result<Program, SourceError> {
        let source = std::fs::read_to_string(&entry).map_err(|e| {
            SourceError::inexistant_module(
                e,
                entry.to_path_buf(),
                importer.to_path_buf(),
                Span { start: 0, end: 0 },
            )
        })?;
        on_file_loaded(&entry, &source);
        let tokens =
            Lexer::tokenize(&source).map_err(|e| SourceError::lexing(e, entry.to_path_buf()))?;
        let program = Parser::new(
            tokens,
            &self.symbols,
            &self.expressions,
            &self.statements,
            &self.types,
        )
        .parse_declarations();
        let program = program.map_err(|e| SourceError::parsing(e, entry.to_path_buf()))?;
        Ok(program)
    }

    ///Resolves the given `entry` path with the given module names. Returns the resultant path, and if it's a folder or a file. True for folders, false for File
    fn resolve_ast_path(
        &self,
        module_names: &[SymbolPointer],
        entry: &Path,
    ) -> Result<(PathBuf, bool), std::io::Error> {
        let mut entry = entry.to_path_buf();
        let mut folder = module_names.last().is_none();
        if let Some((last, module_names)) = module_names.split_last() {
            for name in module_names {
                entry.push(self.symbols.get_name(*name));
            }
            entry.push(self.symbols.get_name(*last));

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
        &self,
        path: &ASTPath,
        global_entry: &Path,
        current: &Path,
        std_path: &Path,
    ) -> Result<(PathBuf, bool), std::io::Error> {
        let path = &path.module_names;
        match self.symbols.get_name(path[0]) {
            "root" => self.resolve_ast_path(&path[1..], global_entry),
            "std" => self.resolve_ast_path(&path[1..], std_path),
            _ => self.resolve_ast_path(path, current),
        }
    }

    ///Resolves the folder imports into the `out` vector. The `resolved` is the resolved directory path to analyze. `generator` is the path to the file that originated this 'request', and `import` is the ast definition that originated so
    fn resolve_folder_imports(
        out: &mut Vec<PathBuf>,
        resolved: &Path,
        generator: &Path,
        import: &FileImport,
    ) -> Result<(), SourceError> {
        let entries = std::fs::read_dir(&resolved).map_err(|e| {
            SourceError::inexistant_module(
                e,
                resolved.to_path_buf(),
                generator.to_path_buf(),
                import.span,
            )
        })?;
        for dir_entry in entries {
            let path = dir_entry
                .map_err(|e| {
                    SourceError::inexistant_module(
                        e,
                        resolved.to_path_buf(),
                        generator.to_path_buf(),
                        import.span,
                    )
                })?
                .path();
            out.push(path);
        }
        Ok(())
    }

    fn load_file_module(
        &self,
        mut entry: PathBuf,
        global_entry: &Path,
        std_path: &Path,
        id: FileId,
        importer: &Path,
        on_file_loaded: &mut impl FnMut(&Path, &str),
    ) -> Result<SourceInfo, SourceError> {
        let generator = entry.clone();
        let program = self.read_file(&entry, importer, on_file_loaded)?;

        entry.pop(); //since its a file, we need to track its current folder to be able to get the siblings, and so we pop the name
        let mut pending: Vec<Vec<PathBuf>> = Vec::new();
        for import in program.imports() {
            let (resolved, is_folder) = self
                .resolve_path(&import.path, global_entry, &entry, std_path)
                .map_err(|e| {
                    SourceError::inexistant_module(e, entry.clone(), generator.clone(), import.span)
                })?;
            let mut import_paths = Vec::new();
            if is_folder {
                Self::resolve_folder_imports(&mut import_paths, &resolved, &generator, import)?;
            } else {
                import_paths.push(resolved);
            }
            pending.push(import_paths);
        }

        Ok(SourceInfo::new(SourceNode::new(id, program), pending))
    }

    pub fn load(
        self,
        entry: PathBuf,
        std_path: PathBuf,
        on_file_loaded: &mut impl FnMut(&Path, &str),
    ) -> Result<Modules, SourceError> {
        let mut global_entry = entry.clone();
        global_entry.pop(); //pops cause the entry should be a file

        let mut pending_entries = {
            let mut out = VecDeque::new();
            out.push_back((entry, None));
            out
        };
        let mut modules: Vec<SourceNode> = Vec::new();
        let mut module_paths: Vec<PathBuf> = Vec::new(); // parallel to modules
        let mut last_id = FileId::from_raw(0);
        let mut entry_index = 0;
        let mut path_to_id: HashMap<PathBuf, FileId> = HashMap::new();
        while let Some((entry, parent)) = pending_entries.pop_front() {
            let importer = match parent {
                Some((mod_idx, _)) => (&module_paths[mod_idx] as &PathBuf).clone(),
                None => entry.clone(),
            };
            let canonical = std::fs::canonicalize(&entry).unwrap_or_else(|_| entry.clone());
            if let Some(&existing_id) = path_to_id.get(&canonical) {
                if let Some((mod_idx, imp_idx)) = parent {
                    (&mut modules[mod_idx] as &mut SourceNode)
                        .import_submodule(imp_idx, existing_id);
                }
                continue;
            }
            path_to_id.insert(canonical, last_id);
            let module_path = entry.clone();
            let module = self.load_file_module(
                entry,
                &global_entry,
                &std_path,
                last_id,
                &importer,
                on_file_loaded,
            )?;
            module_paths.push(module_path);
            // Pre-allocate per-import slot for this new module's imports
            let import_count = module.pending_count();
            for (imp_idx, import_paths) in module.pending.into_iter().enumerate() {
                for path in import_paths {
                    pending_entries.push_back((path, Some((entry_index, imp_idx))));
                }
            }
            let mut node = module.module;
            node.import_submodules = vec![Vec::new(); import_count];
            if let Some((mod_idx, imp_idx)) = parent {
                modules[mod_idx].import_submodule(imp_idx, last_id);
            }
            modules.push(node);
            last_id = last_id.next();
            entry_index += 1;
        }

        Ok(Modules {
            loader: self,
            modules,
        })
    }
}
