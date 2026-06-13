use std::path::PathBuf;

fn compile_ok(path: &str) {
    let result = slynx::compile_to_ir(PathBuf::from(path), None);
    assert!(
        result.is_ok(),
        "compilation failed for {path}:\n{:?}",
        result.err().unwrap(),
    );
}

/// File import with `using … as …` alias on a single usage.
#[test]
fn test_file_import_with_alias() {
    compile_ok("examples/imports/main.slx");
}

/// Selective import with brace syntax: `import path using {Name}`.
#[test]
fn test_brace_select_import() {
    compile_ok("examples/imports/brace_import.slx");
}

/// Selective import with brace syntax and aliases: `import path using {Name as Alias}`.
#[test]
fn test_brace_alias_import() {
    compile_ok("examples/imports/brace_alias_import.slx");
}

/// Importing a name that exists in the workspace but not in the specified module must error.
#[test]
fn import_using_name_not_in_specified_module_errors() {
    // BgGreen lives in another.slx, not styles.slx — must not resolve across modules
    let result = slynx::compile_to_ir(
        PathBuf::from("examples/imports/test_wrong_module_import.slx"),
        None,
    );
    assert!(
        result.is_err(),
        "should error: BgGreen is not exported by styles.slx"
    );
}
