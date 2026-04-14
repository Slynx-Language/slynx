use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use slynx::{SlynxContext, compile_code};

fn temp_case_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    path.push(format!("slynx-{name}-{}-{nonce}", std::process::id()));
    path
}

fn write_temp_source(case_dir: &PathBuf) -> PathBuf {
    fs::create_dir_all(case_dir).expect("temp case dir should be created");
    let source_path = case_dir.join("input.slynx");
    let source = fs::read_to_string("slynx/booleans.slynx").expect("fixture should exist");
    fs::write(&source_path, source).expect("temp source should be written");
    source_path
}

#[test]
fn compile_returns_output_before_writing() {
    let case_dir = temp_case_dir("compile-output");
    let source_path = write_temp_source(&case_dir);
    let output_path = source_path.with_extension("sir");

    let context = SlynxContext::new(Arc::new(source_path)).expect("context should be created");
    let output = context.compile().expect("compilation should succeed");

    assert_eq!(output.output_path(), output_path.as_path());
    assert!(!output_path.exists());

    output.write().expect("output should be written");
    assert!(output_path.exists());

    fs::remove_dir_all(case_dir).expect("temp case dir should be removed");
}

#[test]
fn compile_code_still_writes_js_output() {
    let case_dir = temp_case_dir("compile-code");
    let source_path = write_temp_source(&case_dir);
    let output_path = source_path.with_extension("sir");

    compile_code(source_path).expect("compile_code should still write the output file");

    assert!(output_path.exists());
    assert!(
        !fs::read_to_string(&output_path)
            .expect("generated output should be readable")
            .is_empty()
    );

    fs::remove_dir_all(case_dir).expect("temp case dir should be removed");
}
