mod common;

use std::{
    fs,
    path::{Path, PathBuf},
};

use slynx::SlynxContext;

fn temp_source_path(case: &str) -> PathBuf {
    let case_dir = common::temp_dir(case);
    let source_path = case_dir.join("input.slynx");
    let source = fs::read_to_string("examples/booleans.syx").expect("fixture should exist");
    fs::write(&source_path, source).expect("temp source should be written");
    source_path
}

fn cleanup(path: &Path) {
    fs::remove_dir_all(path.parent().expect("temp file should live in a case dir"))
        .expect("temp case dir should be removed");
}

#[test]
fn compile_returns_output_before_writing() {
    let source_path = temp_source_path("compile-output");
    let output_path = source_path.with_extension("sir");

    let context = SlynxContext::new(source_path, None).expect("context should be created");
    let output = context.compile().expect("compilation should succeed");

    assert_eq!(output.output_path(), output_path.as_path());
    assert!(!output_path.exists());

    output.write().expect("output should be written");
    assert!(output_path.exists());

    cleanup(&output_path);
}

#[test]
fn build_stages_exposes_hir_and_ir_dumps_without_writing_files() {
    let source_path = temp_source_path("build-stages");
    let hir_path = source_path.with_extension("hir");
    let ir_path = source_path.with_extension("ir");

    let context = SlynxContext::new(source_path, None).expect("context should be created");
    let stages = context.build_stages().expect("stages should build");

    assert_eq!(stages.dump_path("hir"), hir_path);
    assert_eq!(stages.dump_path("ir"), ir_path);

    assert!(!stages.ir_text().is_empty());
    assert!(!hir_path.exists());
    assert!(!ir_path.exists());

    cleanup(&hir_path);
}

#[test]
fn build_stages_can_write_hir_ir_and_sir_outputs() {
    let source_path = temp_source_path("dump-files");

    let ir_path = source_path.with_extension("ir");
    let sir_path = source_path.with_extension("sir");

    let context = SlynxContext::new(source_path, None).expect("context should be created");
    let stages = context.build_stages().expect("stages should build");

    stages.write_ir().expect("ir dump should be written");
    let output = stages.into_output();
    output.write().expect("sir output should be written");

    for path in [&ir_path, &sir_path] {
        assert!(path.exists(), "{} should exist", path.display());
        assert!(
            !fs::read_to_string(path)
                .expect("generated dump should be readable")
                .is_empty()
        );
    }

    cleanup(&ir_path);
}
