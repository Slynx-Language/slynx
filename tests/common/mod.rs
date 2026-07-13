#![allow(dead_code)]

use std::path::PathBuf;

use slynx::SlynxContext;
use slynx_ir::SlynxIR;
pub fn load_source(source: &str) -> SlynxContext {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("slynx-source-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir should be creatable");
    let path = dir.join("test.syx");
    std::fs::write(&path, source).expect("source should be written");
    SlynxContext::new(path, Some(PathBuf::from("./lib/std")))
        .expect("context should be created from temp file")
}
pub fn load_context(path: &str) -> SlynxContext {
    SlynxContext::new(path.into(), None).expect("Context should generate")
}
pub static STD_PATH: std::sync::LazyLock<PathBuf> =
    std::sync::LazyLock::new(|| PathBuf::from("lib/std"));
pub fn compile_ok(path: &str) -> SlynxIR {
    let result = slynx::compile_to_ir(PathBuf::from(path), Some(STD_PATH.clone()));

    assert!(
        result.is_ok(),
        "compilation failed for {path}:\n{:?}",
        result.err().unwrap(),
    );
    result.unwrap()
}

/// Compiles inline source code through the full pipeline.
/// Returns `Ok(())` on success or the formatted error message on failure.
pub fn compile_source(source: &str) -> std::result::Result<(), String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("slynx-checker-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("test.syx");
    std::fs::write(&path, source).map_err(|e| e.to_string())?;
    let context = slynx::SlynxContext::new(path, None).map_err(|e| e.to_string())?;
    match context.compile() {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
