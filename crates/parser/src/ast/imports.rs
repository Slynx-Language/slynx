use common::Span;

use crate::SymbolPointer;

#[derive(Debug)]
pub struct ASTPath {
    pub module_names: Vec<SymbolPointer>,
}
#[derive(Debug)]
pub struct ImportUsage {
    pub content_name: SymbolPointer,
    pub alias: Option<SymbolPointer>,
}

#[derive(Debug)]
pub struct FileImport {
    pub path: ASTPath,
    pub usages: Vec<ImportUsage>,
    pub span: Span,
}
