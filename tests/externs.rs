use std::path::PathBuf;
mod common;

/// Discovers and compiles every `.slx` example in `tests/examples/externs/`.
/// Each file is a focused test case for a specific extern declaration feature.
#[test]
fn extern_examples() {
    let dir = PathBuf::from("examples/externs");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("extern examples directory should exist at examples/externs/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "slx"))
        .collect();
    entries.sort_by_key(|e| e.path());

    assert!(
        !entries.is_empty(),
        "no .slx files found in tests/examples/externs/"
    );

    let mut failures = Vec::new();
    for entry in &entries {
        let path = entry.path();
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();

        match slynx::compile_to_ir(path.clone(), Some(common::STD_PATH.clone())) {
            Ok(_) => {
                println!("PASS: {name}");
            }
            Err(e) => {
                println!("FAIL: {name}: {e:?}");
                failures.push(name);
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} extern example(s) failed to compile:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}
