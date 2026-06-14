#[derive(Debug)]
pub struct ASTPath {
    pub module_names: Vec<String>,
}
#[derive(Debug)]
pub struct ImportUsage {
    pub content_name: String,
    pub alias: Option<String>,
}

#[derive(Debug)]
pub struct FileImport {
    pub path: ASTPath,
    pub usages: Vec<ImportUsage>,
}
