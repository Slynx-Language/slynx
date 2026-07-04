#![allow(dead_code)]

use std::path::{Path, PathBuf};

use slynx::SlynxContext;
use slynx_ir::SlynxIR;

pub fn load_source(source: &str) -> SlynxContext {
    SlynxContext::from_source(source.to_string(), Path::new("input.slx"))
}

pub fn load_hir(path: &str) -> SlynxContext {
    SlynxContext::new(PathBuf::from(path), Some(STD_PATH.clone()))
        .expect("context should be created")
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
