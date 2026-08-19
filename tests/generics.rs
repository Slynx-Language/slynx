use std::path::{Path, PathBuf};
mod common;

/// The expected outcome of an example file, declared with a marker comment:
///   - `// xfail: <reason>` → the file must currently FAIL to compile (a known
///     limitation or a correctly rejected error case). Once the underlying
///     issue is fixed, remove or downgrade the marker to a plain example.
///   - `// xpass: <reason>` → the file must currently COMPILE even though it
///     should eventually be rejected (documents a missing validation).
///   - no marker → a regular positive example that must compile.
#[derive(PartialEq, Clone, Copy)]
enum Expect {
    Pass,
    Fail,
}

fn read_expectation(path: &Path) -> (Expect, Option<String>) {
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

/// Discovers and compiles every `.slx` example in `examples/generics/`.
/// Each file is a focused test case for a generics feature. Files marked with
/// `// xfail:` are expected to be rejected (or to panic) today; any change in
/// their outcome is reported so the marker can be updated.
#[test]
fn generic_examples() {
    let dir = PathBuf::from("examples/generics");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("generics examples directory should exist at examples/generics/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "slx"))
        .collect();
    entries.sort_by_key(|e| e.path());

    assert!(
        !entries.is_empty(),
        "no .slx files found in examples/generics/"
    );

    let mut failures = Vec::new();

    for entry in &entries {
        let path = entry.path();
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        let (expect, reason) = read_expectation(&path);

        // A `// xfail:` example may still crash the compiler; treat a panic as
        // a (rejected) outcome instead of aborting the whole test suite.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            slynx::compile_to_ir(path.clone(), Some(common::STD_PATH.clone()))
                .map(|ir| ir.format_sir())
                .map_err(|e| format!("{e:?}"))
        }));

        let (compiled, detail) = match outcome {
            Ok(Ok(ir)) => (true, ir),
            Ok(Err(e)) => (false, e),
            Err(_) => (false, "<panicked during compilation>".to_string()),
        };

        match (expect, compiled) {
            (Expect::Pass, true) => {
                eprintln!("PASS: {name};");
                eprintln!("IR Generated:\n{detail}");
            }
            (Expect::Pass, false) => {
                println!("FAIL: {name}: {detail}");
                failures.push(format!("{name} should compile but was rejected:\n{detail}"));
            }
            (Expect::Fail, true) => {
                let reason = reason.as_deref().unwrap_or("no reason given");
                println!(
                    "EXPECTED TO FAIL, BUT INSTEAD PASSED, FIXED: {name} compiled but was marked xfail ({reason})"
                );
                failures.push(format!(
                    "{name} was marked xfail but now compiles ({reason})"
                ));
            }
            (Expect::Fail, false) => {
                eprintln!("FAILED TO COMPILE: {name};");
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} generics example(s) failed:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}
