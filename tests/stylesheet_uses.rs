use std::path::PathBuf;

/// Compiles a single .slx file and expects success.
fn compile_slx(path: PathBuf) {
    let result = slynx::compile_to_ir(path.clone(), None);
    assert!(
        result.is_ok(),
        "compilation failed for {}:\n{:?}",
        path.display(),
        result.err().unwrap(),
    );
}

#[test]
fn all_stylesheet_uses() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/styles");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("examples/styles should exist")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "slx"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        compile_slx(path);
    }
}
