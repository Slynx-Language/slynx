use std::{collections::VecDeque, path::PathBuf};

use slynx_lexer::{Lexer, LexerError};
use slynx_parser::{ASTDeclaration, ASTDeclarationKind, ASTPath, ParseError, Parser};

#[derive(Debug, Clone, Copy)]
///An ID to represent a file
pub struct FileId(u64);
impl FileId {
    pub fn as_raw(&self) -> u64 {
        self.0
    }
    fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

pub enum Module {
    Folder(FolderModule),
    File(FileModule),
}

pub struct FolderModule {
    modules: Vec<Module>,
}

///A module that was parsed
pub struct FileModule {
    ///The path to the file that generated this module
    pub id: FileId,
    ///The declarations inside this module
    pub declarations: Vec<ASTDeclaration>,
    ///The id of files that are submodules of this
    pub submodules: Vec<FileId>,
}

#[derive(Debug)]
pub enum ModuleErrorKind {
    InexsitantModule(std::io::Error),
    Lexing(LexerError),
    Parsing(ParseError),
}

#[derive(Debug)]
pub struct ModuleError {
    kind: ModuleErrorKind,
    entry: PathBuf,
}

impl ModuleError {
    pub fn inexistant_module(e: std::io::Error, entry: PathBuf) -> Self {
        Self {
            kind: ModuleErrorKind::InexsitantModule(e),
            entry: entry.clone(),
        }
    }
    pub fn lexing(e: LexerError, entry: PathBuf) -> Self {
        Self {
            kind: ModuleErrorKind::Lexing(e),
            entry: entry.clone(),
        }
    }
    pub fn parsing(e: ParseError, entry: PathBuf) -> Self {
        Self {
            kind: ModuleErrorKind::Parsing(e),
            entry: entry.clone(),
        }
    }
}

impl FileModule {
    pub fn new(id: FileId, declarations: Vec<ASTDeclaration>) -> Self {
        Self {
            id,
            declarations,
            submodules: Vec::new(),
        }
    }
}

pub struct ModuleLoader;

pub struct ModuleInfo {
    module: FileModule,
    pending: Vec<PathBuf>,
}

impl ModuleLoader {
    ///Resolves the given `entry` path with the given module names. Returns the resultant path, and if it's a folder or a file. True for folders, false for File
    fn resolve_ast_path(
        module_names: &[String],
        entry: &PathBuf,
    ) -> Result<(PathBuf, bool), std::io::Error> {
        let mut entry = entry.clone();
        let mut folder = false;
        if let Some((last, module_names)) = module_names.split_last() {
            for name in module_names {
                entry.push(name);
            }
            entry.push(last);
            folder = entry.metadata()?.is_dir();
            if !folder {
                entry.set_extension("slx");
            };
        }
        Ok((entry, folder))
    }

    fn resolve_path(
        path: &ASTPath,
        global_entry: &PathBuf,
        current: &PathBuf,
    ) -> Result<(PathBuf, bool), std::io::Error> {
        let path = &path.module_names;
        if path[0] == "root" {
            Self::resolve_ast_path(&path[1..], global_entry)
        } else {
            Self::resolve_ast_path(path, current)
        }
    }

    fn load_file_module(
        entry: PathBuf,
        global_entry: &PathBuf,
        id: FileId,
    ) -> Result<ModuleInfo, ModuleError> {
        let source = std::fs::read_to_string(&entry)
            .map_err(|e| ModuleError::inexistant_module(e, entry.clone()))?;
        let tokens = Lexer::tokenize(&source).map_err(|e| ModuleError::lexing(e, entry.clone()))?;
        let declarations = Parser::new(tokens)
            .parse_declarations()
            .map_err(|e| ModuleError::parsing(e, entry.clone()))?;
        let mut submodules = Vec::new();
        for decl in &declarations {
            if let ASTDeclarationKind::Import(ref import) = decl.kind {
                let inexistant_err = |e| ModuleError::inexistant_module(e, entry.clone());
                let (entry, is_folder) = Self::resolve_path(&import.path, global_entry, &entry)
                    .map_err(inexistant_err)?;

                if is_folder {
                    for pending in std::fs::read_dir(entry).map_err(inexistant_err)? {
                        let pending = pending.map_err(inexistant_err)?.path();
                        submodules.push(pending);
                    }
                } else {
                    submodules.push(entry);
                }
            }
        }
        Ok(ModuleInfo {
            module: FileModule {
                id,
                declarations,
                submodules: Vec::new(),
            },
            pending: submodules,
        })
    }

    pub fn load(entry: PathBuf) -> Result<Vec<FileModule>, ModuleError> {
        let global_entry = entry.clone();
        let mut pending_entries = VecDeque::new();
        pending_entries.push_back((entry, None));
        let mut modules: Vec<FileModule> = Vec::new();
        let mut last_id = FileId(0);
        let mut entry_index = 0;
        while let Some((entry, index)) = pending_entries.pop_front() {
            let module = Self::load_file_module(entry, &global_entry, last_id)?;
            for pending in module.pending {
                pending_entries.push_back((pending, Some(entry_index)));
            }
            if let Some(index) = index {
                modules[index].submodules.push(last_id);
            }
            modules.push(module.module);
            last_id = last_id.next();
            entry_index += 1;
        }

        Ok(modules)
    }
}
