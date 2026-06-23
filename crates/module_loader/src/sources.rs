use std::{ops::Deref, path::PathBuf};

use slynx_parser::Program;

#[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq)]
///An ID to represent a file
pub struct FileId(u32);
impl FileId {
    pub fn from_raw(value: u32) -> Self {
        Self(value)
    }
    pub fn as_raw(&self) -> u32 {
        self.0
    }
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

pub struct SourceInfo {
    pub(crate) module: SourceNode,
    /// Per-import pending paths: pending[i] are the paths for the i-th import declaration.
    pub(crate) pending: Vec<Vec<PathBuf>>,
}

impl SourceInfo {
    pub fn new(module: SourceNode, pending: Vec<Vec<PathBuf>>) -> Self {
        Self { module, pending }
    }
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[derive(Debug)]
///A module that was parsed
pub struct SourceNode {
    ///The path to the file that generated this module
    pub id: FileId,
    ///The declarations inside this module
    pub program: Program,
    ///Per-import submodule lists: `import_submodules[i]` contains the FileIds for the i-th import declaration.
    pub import_submodules: Vec<Vec<FileId>>,
}

impl SourceNode {
    pub fn new(id: FileId, program: Program) -> Self {
        Self {
            id,
            program,
            import_submodules: Vec::new(),
        }
    }
    ///Imports the given `file` into the import at the given `import_index`
    pub(crate) fn import_submodule(&mut self, import_index: usize, file: FileId) {
        self.import_submodules[import_index].push(file);
    }
}

impl Deref for SourceNode {
    type Target = Program;
    fn deref(&self) -> &Self::Target {
        &self.program
    }
}
