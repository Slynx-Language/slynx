use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_case_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    path.push(format!("slynx-cli-{name}-{}-{nonce}", std::process::id()));
    path
}

fn write_temp_source(case_dir: &PathBuf) -> PathBuf {
    // Copy a real fixture into a temp directory so CLI output files do not touch the repository.
    fs::create_dir_all(case_dir).expect("temp case dir should be created");
    let source_path = case_dir.join("input.slynx");
    let source = fs::read_to_string("slynx/booleans.slynx").expect("fixture should exist");
    fs::write(&source_path, source).expect("temp source should be written");
    source_path
}

fn run_cli(args: &[&str]) {
    let status = Command::new(env!("CARGO_BIN_EXE_slynx"))
        .args(args)
        .status()
        .expect("cli should be executable");
    assert!(status.success(), "cli should exit successfully");
}

#[test]
fn cli_writes_sir_by_default() {
    let case_dir = temp_case_dir("sir-default");
    let source_path = write_temp_source(&case_dir);
    let sir_path = source_path.with_extension("sir");

    let source_arg = source_path.to_string_lossy().into_owned();
    run_cli(&[&source_arg]);

    assert!(sir_path.exists(), "{} should exist", sir_path.display());
    assert!(
        !fs::read_to_string(&sir_path)
            .expect("sir output should be readable")
            .is_empty()
    );

    fs::remove_dir_all(case_dir).expect("temp case dir should be removed");
}

#[test]
fn cli_writes_optional_hir_and_ir_dumps() {
    let case_dir = temp_case_dir("stage-dumps");
    let source_path = write_temp_source(&case_dir);
    let hir_path = source_path.with_extension("hir");
    let ir_path = source_path.with_extension("ir");
    let sir_path = source_path.with_extension("sir");

    let source_arg = source_path.to_string_lossy().into_owned();
    run_cli(&["--target", &source_arg, "--hir", "--ir"]);

    for path in [&hir_path, &ir_path, &sir_path] {
        assert!(path.exists(), "{} should exist", path.display());
        assert!(
            !fs::read_to_string(path)
                .expect("generated dump should be readable")
                .is_empty()
        );
    }

    fs::remove_dir_all(case_dir).expect("temp case dir should be removed");
}
