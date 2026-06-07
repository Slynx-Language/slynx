use std::{collections::VecDeque, path::PathBuf};

use slynx_lexer::{Lexer, LexerError};
use slynx_parser::{ASTDeclaration, ASTDeclarationKind, ASTPath, ParseError, Parser};

#[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq)]
///An ID to represent a file
pub struct FileId(u32);
impl FileId {
    pub fn as_raw(&self) -> u32 {
        self.0
    }
    fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Debug)]
///A module that was parsed
pub struct SourceNode {
    ///The path to the file that generated this module
    pub id: FileId,
    ///The declarations inside this module
    pub declarations: Vec<ASTDeclaration>,
    ///The id of files that are submodules of this
    pub submodules: Vec<FileId>,
}

#[derive(Debug)]
pub enum SourceErrorKind {
    InexsitantSource(std::io::Error),
    Lexing(LexerError),
    Parsing(ParseError),
}

#[derive(Debug)]
pub struct SourceError {
    kind: SourceErrorKind,
    entry: PathBuf,
}

impl SourceError {
    pub fn inexistant_module(e: std::io::Error, entry: PathBuf) -> Self {
        Self {
            kind: SourceErrorKind::InexsitantSource(e),
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
}

impl SourceNode {
    pub fn new(id: FileId, declarations: Vec<ASTDeclaration>) -> Self {
        Self {
            id,
            declarations,
            submodules: Vec::new(),
        }
    }
}

pub struct SourceLoader {
    ///Where the path loader is being initializing at
    root: PathBuf,
}

pub struct SourceInfo {
    module: SourceNode,
    pending: Vec<PathBuf>,
}

impl SourceLoader {
    ///Resolves the given `entry` path with the given module names. Returns the resultant path, and if it's a folder or a file. True for folders, false for File
    fn resolve_ast_path(
        module_names: &[String],
        entry: &PathBuf,
    ) -> Result<(PathBuf, bool), std::io::Error> {
        let mut entry = entry.clone();
        let mut folder = false;
        println!("{module_names:?} {entry:?}");
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
        println!("gay");
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
        mut entry: PathBuf,
        global_entry: &PathBuf,
        id: FileId,
    ) -> Result<SourceInfo, SourceError> {
        let source = std::fs::read_to_string(&entry)
            .map_err(|e| SourceError::inexistant_module(e, entry.clone()))?;
        let tokens = Lexer::tokenize(&source).map_err(|e| SourceError::lexing(e, entry.clone()))?;
        let declarations = Parser::new(tokens)
            .parse_declarations()
            .map_err(|e| SourceError::parsing(e, entry.clone()))?;
        entry.pop(); //since its a file, we need to track its current folder to be able to get the siblings, and so we pop the name
        let mut submodules = Vec::new();
        for decl in &declarations {
            if let ASTDeclarationKind::Import(ref import) = decl.kind {
                let (entry, is_folder) = Self::resolve_path(&import.path, global_entry, &entry)
                    .map_err(|e| SourceError::inexistant_module(e, entry.clone()))?;
                println!("{entry:?} {is_folder}");
                if is_folder {
                    for pending in std::fs::read_dir(&entry)
                        .map_err(|e| SourceError::inexistant_module(e, entry.clone()))?
                    {
                        let pending = pending
                            .map_err(|e| SourceError::inexistant_module(e, entry.clone()))?
                            .path();
                        submodules.push(pending);
                    }
                } else {
                    submodules.push(entry);
                }
            }
        }
        Ok(SourceInfo {
            module: SourceNode {
                id,
                declarations,
                submodules: Vec::new(),
            },
            pending: submodules,
        })
    }

    pub fn load(entry: PathBuf) -> Result<Vec<SourceNode>, SourceError> {
        let mut global_entry = entry.clone();
        global_entry.pop(); //pops cause the entry should be a file
        let mut pending_entries = VecDeque::new();
        pending_entries.push_back((entry, None));
        let mut modules: Vec<SourceNode> = Vec::new();
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
