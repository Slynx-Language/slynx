//! Shared test harness for the integration tests under `tests/`.
//!
//! Everything here is "auxiliary machinery" that the individual test binaries
//! reuse so that each test only describes *what* it asserts, never *how* a
//! file is located, compiled, or turned into an example-directory corpus.
//!
//! Two families of helpers are provided:
//!
//! - **Single-file testing** (`load_source`, `load_example`, `compile_ok`,
//!   `compile_ok_sir`, `compile_fails`, `compile_source*`): compile one inline
//!   source or one example file and assert the outcome.
//! - **Folder testing** (`example_files`, `read_expectation`,
//!   `run_examples`, `run_examples_ok`): drive a whole directory of example
//!   files, honoring the `// xfail:` / `// xpass:` marker convention that the
//!   behavior matrix uses (`docs/contributing/extensible-core/09-behavior-matrix.md`).

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use slynx::SlynxContext;
use slynx_hir::SlynxHir;
use slynx_ir::SlynxIR;

/// Path to the standard prelude used by every example corpus.
pub static STD_PATH: std::sync::LazyLock<PathBuf> =
    std::sync::LazyLock::new(|| PathBuf::from("lib/std"));

/// Creates a uniquely named temporary directory and returns its path.
pub fn temp_dir(kind: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("slynx-{kind}-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir should be creatable");
    dir
}

/// Writes `source` into a unique temporary file and returns its path.
fn write_temp_source(kind: &str, source: &str) -> PathBuf {
    let path = temp_dir(kind).join("test.syx");
    std::fs::write(&path, source).expect("source should be written");
    path
}

/// Creates a context over `source` written to a unique temp file.
pub fn load_source(source: &str) -> SlynxContext {
    let path = write_temp_source("slynx-source", source);
    SlynxContext::new(path, Some(STD_PATH.clone()))
        .expect("context should be created from temp file")
}

/// Creates a context over the example file at `path` (relative to the repo root).
pub fn load_example(path: &str) -> SlynxContext {
    SlynxContext::new(PathBuf::from(path), Some(STD_PATH.clone()))
        .expect("context should be created from example file")
}

/// Creates a context over the file at `path` without attaching the std prelude.
pub fn load_context(path: &str) -> SlynxContext {
    SlynxContext::new(path.into(), None).expect("Context should generate")
}

/// Compiles the example at `path` and returns its IR.
pub fn compile_ok(path: &str) -> SlynxIR {
    slynx::compile_to_ir(PathBuf::from(path), Some(STD_PATH.clone()))
        .unwrap_or_else(|e| panic!("compilation failed for {path}:\n{e:?}"))
}

/// Compiles the example at `path`, asserting the produced output targets a
/// `.sir` path — the default materialized artifact of the pipeline.
pub fn compile_ok_sir(path: &str) -> SlynxIR {
    let output = load_example(path)
        .compile()
        .expect("example should compile");
    assert_eq!(
        output
            .output_path()
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("sir"),
        "output path for {path} should end in `.sir`"
    );
    output.ir()
}

/// Compiles the example at `path`, expecting rejection; returns the error detail.
pub fn compile_fails(path: &str) -> String {
    match slynx::compile_to_ir(PathBuf::from(path), Some(STD_PATH.clone())) {
        Ok(_) => panic!("compilation unexpectedly succeeded for {path}"),
        Err(e) => format!("{e:?}"),
    }
}

/// Compiles inline `source` through the full pipeline.
/// Returns `Ok(())` on success or the formatted error message on failure.
pub fn compile_source(source: &str) -> Result<(), String> {
    let context = SlynxContext::new(write_temp_source("slynx-checker", source), None)
        .map_err(|e| e.to_string())?;
    match context.compile() {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Asserts that `source` compiles cleanly.
pub fn compile_source_ok(source: &str) {
    compile_source(source).unwrap_or_else(|err| panic!("expected source to compile:\n{err}"));
}

/// Asserts that `source` fails to compile and returns the formatted error.
pub fn compile_source_err(source: &str) -> String {
    compile_source(source).expect_err("expected source to be rejected")
}

/// Asserts that `source` fails to compile and that the error mentions `needle`.
/// Returns the error for further assertions.
pub fn compile_source_err_contains(source: &str, needle: &str) -> String {
    let err = compile_source_err(source);
    assert!(
        err.contains(needle),
        "expected error to contain {needle:?}, got:\n{err}"
    );
    err
}

// ─── Example-directory harness ───────────────────────────────────────────────

/// The declared expected outcome of an example file, read from a marker comment:
///   - `// xfail: <reason>` → the file must currently FAIL to compile (a known
///     limitation or a correctly rejected error case). Once the underlying
///     issue is fixed, remove or downgrade the marker to a plain example.
///   - `// xpass: <reason>` → the file must currently COMPILE even though it
///     should eventually be rejected (documents a missing validation).
///   - no marker → a regular positive example that must compile.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Expect {
    Pass,
    Fail,
}

/// Reads the first marker comment (if any) out of `path`.
pub fn read_expectation(path: &Path) -> (Expect, Option<String>) {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    for line in content.lines() {
        let line = line.trim();
        if let Some(reason) = line.strip_prefix("// xfail:") {
            return (Expect::Fail, Some(reason.trim().to_string()));
        }
        if let Some(reason) = line.strip_prefix("// xpass:") {
            return (Expect::Pass, Some(reason.trim().to_string()));
        }
    }
    (Expect::Pass, None)
}

/// The sorted list of every `.slx`/`.syx` example file inside `dir`.
pub fn example_files(dir: &str) -> Vec<PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("example directory should exist at {dir}: {e}"))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "slx" || ext == "syx")
        })
        .map(|e| e.path())
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "no example files found in {dir}");
    entries
}

/// Compiles a single example file, catching panics. Returns `(compiled, detail)`:
/// `detail` is the SIR text on success or the formatted error on failure. A
/// `// xfail:` example may still crash the compiler; a panic is treated as a
/// (rejected) outcome instead of aborting the whole test suite.
pub fn try_compile_example(path: &Path) -> (bool, String) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        slynx::compile_to_ir(path.to_path_buf(), Some(STD_PATH.clone()))
            .map(|ir| ir.format_sir())
            .map_err(|e| format!("{e:?}"))
    }));
    match outcome {
        Ok(Ok(ir)) => (true, ir),
        Ok(Err(e)) => (false, e),
        Err(_) => (false, "<panicked during compilation>".to_string()),
    }
}

/// Runs the marker-driven harness over every example file in `dir` and fails the
/// test if any file's outcome disagrees with its declared marker. `label` is used
/// only in failure messages (e.g. "generics").
pub fn run_examples(dir: &str, label: &str) {
    let mut failures = Vec::new();
    for path in example_files(dir) {
        let name = file_stem(&path);
        let (expect, reason) = read_expectation(&path);
        let (compiled, detail) = try_compile_example(&path);
        match (expect, compiled) {
            (Expect::Pass, true) => eprintln!("PASS: {name}"),
            (Expect::Pass, false) => {
                failures.push(format!("{name} should compile but was rejected:\n{detail}"));
            }
            (Expect::Fail, true) => {
                failures.push(format!(
                    "{name} was marked xfail ({}) but now compiles",
                    reason.as_deref().unwrap_or("no reason given")
                ));
            }
            (Expect::Fail, false) => eprintln!("REJECTED AS EXPECTED: {name}"),
        }
    }
    if !failures.is_empty() {
        panic!(
            "{} {label} example(s) failed:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}

/// Compiles every example file in `dir` and asserts that each one compiles
/// (no marker comments are honored). `label` is used only in failure messages.
pub fn run_examples_ok(dir: &str, label: &str) {
    let mut failures = Vec::new();
    for path in example_files(dir) {
        let name = file_stem(&path);
        match slynx::compile_to_ir(path.clone(), Some(STD_PATH.clone())) {
            Ok(_) => eprintln!("PASS: {name}"),
            Err(e) => failures.push(format!("{name} failed to compile:\n{e:?}")),
        }
    }
    if !failures.is_empty() {
        panic!(
            "{} {label} example(s) failed:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

/// Counts resolved function declarations whose mangled name starts with `prefix`.
/// Used to assert how many specializations of a generic function were produced.
pub fn count_specializations(hir: &SlynxHir, prefix: &str) -> usize {
    hir.store
        .files
        .iter()
        .map(|file| {
            file.declarations
                .declarations
                .functions
                .iter()
                .filter(|declaration| hir.get_name(declaration.name).starts_with(prefix))
                .count()
        })
        .sum()
}
